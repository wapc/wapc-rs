type Result<T> = std::result::Result<T, wapc::errors::Error>;

use std::sync::Arc;
use std::time::Duration;

use crossbeam::channel::{Receiver as SyncReceiver, SendTimeoutError, Sender as SyncSender};
use rusty_pool::ThreadPool;
use tokio::sync::oneshot::Sender as OneshotSender;
use wapc::WapcHost;

use crate::errors::Error;

/// The [HostPool] initializes a number of workers for the passed [WapcHost] factory function.
///
#[must_use]
pub struct HostPool {
  /// The name of the [HostPool] (for debugging purposes).
  pub name: String,
  pool: Option<ThreadPool>,
  factory: Arc<dyn Fn() -> WapcHost + Send + Sync + 'static>,
  max_threads: usize,
  max_wait: Duration,
  max_idle: Duration,
  tx: SyncSender<WorkerMessage>,
  rx: SyncReceiver<WorkerMessage>,
}

impl std::fmt::Debug for HostPool {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("HostPool")
      .field("name", &self.name)
      .field("tx", &self.tx)
      .field("rx", &self.rx)
      .finish()
  }
}

type WorkerMessage = (
  OneshotSender<std::result::Result<Vec<u8>, wapc::errors::Error>>,
  String,
  Vec<u8>,
);

impl HostPool {
  /// Instantiate a new HostPool.
  pub fn new<N, F>(
    name: N,
    factory: F,
    min_threads: usize,
    max_threads: usize,
    max_wait: Duration,
    max_idle: Duration,
  ) -> Self
  where
    N: AsRef<str>,
    F: Fn() -> WapcHost + Send + Sync + 'static,
  {
    debug!("Creating new wapc host pool with size {}", max_threads);
    let arcfn = Arc::new(factory);
    let pool = rusty_pool::Builder::new()
      .name(name.as_ref().to_owned())
      .core_size(min_threads)
      .max_size(max_threads)
      .keep_alive(Duration::from_millis(0))
      .build();

    let (tx, rx) = crossbeam::channel::bounded::<WorkerMessage>(1);

    let pool = Self {
      name: name.as_ref().to_owned(),
      factory: arcfn,
      pool: Some(pool),
      max_threads,
      max_wait,
      max_idle,
      tx,
      rx,
    };

    for _ in 0..min_threads {
      pool.spawn(None).unwrap();
    }

    pool
  }

  /// Get the current number of active workers.
  #[must_use]
  pub fn num_active_workers(&self) -> usize {
    match &self.pool {
      Some(pool) => pool.get_current_worker_count(),
      None => 0,
    }
  }

  fn spawn(&self, max_idle: Option<Duration>) -> Result<()> {
    match &self.pool {
      Some(pool) => {
        let name = self.name.clone();
        let i = pool.get_current_worker_count();
        let factory = self.factory.clone();
        let rx = self.rx.clone();
        pool.execute(move || {
          trace!("Host thread {}.{} started...", name, i);
          let host = factory();
          loop {
            let message = match max_idle {
              None => rx.recv().map_err(|e| e.to_string()),
              Some(duration) => rx.recv_timeout(duration).map_err(|e| e.to_string()),
            };
            if let Err(e) = message {
              debug!("Host thread {}.{} closing: {}", name, i, e);
              break;
            }
            let (tx, op, payload) = message.unwrap();
            trace!(
              "Host thread {}.{} received call for {} with {} byte payload",
              name,
              i,
              op,
              payload.len()
            );
            let result = host.call(&op, &payload);
            if tx.send(result).is_err() {
              error!("Host thread {}.{} failed when returning a value...", name, i);
            }
          }

          trace!("Host thread {}.{} stopped.", name, i);
        });
        Ok(())
      }
      None => Err(Error::NoPool.into()),
    }
  }

  /// Call an operation on one of the workers.
  pub async fn call<T: AsRef<str> + Sync + Send>(&self, op: T, payload: Vec<u8>) -> Result<Vec<u8>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    // Start the call with a timeout of max_wait.
    let result = match self
      .tx
      .send_timeout((tx, op.as_ref().to_owned(), payload), self.max_wait)
    {
      Ok(_) => Ok(()),
      Err(e) => {
        // If we didn't get a response in time...
        let args = match e {
          SendTimeoutError::Timeout(args) => {
            debug!("Timeout on pool '{}'", self.name);
            args
          }
          SendTimeoutError::Disconnected(args) => {
            warn!("Pool worker disconnected on pool '{}'", self.name);
            args
          }
        };
        // grow the pool...
        if self.num_active_workers() < self.max_threads {
          if let Err(e) = self.spawn(Some(self.max_idle)) {
            error!("Error spawning worker for host pool '{}': {}", self.name, e);
          };
        }
        // ...and wait.
        self.tx.send(args)
      }
    };
    if let Err(e) = result {
      return Err(wapc::errors::Error::General(e.to_string()));
    }
    match rx.await {
      Ok(res) => res,
      Err(e) => Err(wapc::errors::Error::General(e.to_string())),
    }
  }

  /// Shut down the host pool.
  pub fn shutdown(&mut self) -> Result<()> {
    let pool = self
      .pool
      .take()
      .ok_or_else(|| wapc::errors::Error::from(crate::errors::Error::NoPool))?;

    pool.shutdown_join();
    Ok(())
  }
}

#[must_use]
/// Builder for a [HostPool]
pub struct HostPoolBuilder {
  name: Option<String>,
  factory: Option<Box<dyn Fn() -> WapcHost + Send + Sync + 'static>>,
  min_threads: usize,
  max_threads: usize,
  max_wait: Duration,
  max_idle: Duration,
}

impl std::fmt::Debug for HostPoolBuilder {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("HostPoolBuilder")
      .field("name", &self.name)
      .field("factory", if self.factory.is_some() { &"Some(Fn)" } else { &"None" })
      .field("min_threads", &self.min_threads)
      .field("max_threads", &self.max_threads)
      .field("max_wait", &self.max_wait)
      .field("max_idle", &self.max_idle)
      .finish()
  }
}

impl Default for HostPoolBuilder {
  fn default() -> Self {
    Self {
      name: None,
      factory: None,
      min_threads: 1,
      max_threads: 2,
      max_wait: Duration::from_millis(100),
      max_idle: Duration::from_secs(5 * 60),
    }
  }
}

impl HostPoolBuilder {
  /// Instantiate a nnew [HostPoolBuilder] with default settings.
  ///
  /// ```
  /// # use wapc_pool::HostPoolBuilder;
  /// let builder = HostPoolBuilder::new();
  /// ```
  ///
  pub fn new() -> Self {
    Self::default()
  }

  /// Set the name for the HostPool.
  ///
  /// ```
  /// # use wapc_pool::HostPoolBuilder;
  /// let builder = HostPoolBuilder::new().name("My Module");
  /// ```
  ///
  pub fn name<T: AsRef<str>>(mut self, name: T) -> Self {
    self.name = Some(name.as_ref().to_owned());
    self
  }

  /// Set the [WapcHost] generator function to use when spawning new workers.
  ///
  /// ```
  /// # use wapc_pool::HostPoolBuilder;
  /// # use wapc::WapcHost;
  /// # let bytes = std::fs::read("../../wasm/crates/wapc-guest-test/build/wapc_guest_test.wasm").unwrap();
  /// let engine = wasmtime_provider::WasmtimeEngineProvider::new(&bytes, None).unwrap();
  /// let pool = HostPoolBuilder::new()
  ///   .factory(move || {
  ///     let engine = engine.clone();
  ///     WapcHost::new(Box::new(engine), None).unwrap()
  ///   })
  ///   .build();
  /// ```
  ///
  pub fn factory<F>(mut self, factory: F) -> Self
  where
    F: Fn() -> WapcHost + Send + Sync + 'static,
  {
    self.factory = Some(Box::new(factory));
    self
  }

  /// Set the minimum, base number of threads to spawn.
  ///
  /// ```
  /// # use wapc_pool::HostPoolBuilder;
  /// let builder = HostPoolBuilder::new().min_threads(1);
  /// ```
  ///
  pub fn min_threads(mut self, min: usize) -> Self {
    self.min_threads = min;
    self
  }

  /// Set the upper limit on the number of threads to spawn.
  ///
  /// ```
  /// # use wapc_pool::HostPoolBuilder;
  /// let builder = HostPoolBuilder::new().max_threads(5);
  /// ```
  ///
  pub fn max_threads(mut self, max: usize) -> Self {
    self.max_threads = max;
    self
  }

  /// Set the timeout for threads to self-close.
  ///
  /// ```
  /// # use wapc_pool::HostPoolBuilder;
  /// # use std::time::Duration;
  /// let builder = HostPoolBuilder::new().max_idle(Duration::from_secs(60));
  /// ```
  ///
  pub fn max_idle(mut self, timeout: Duration) -> Self {
    self.max_idle = timeout;
    self
  }

  /// Set the maximum amount of time to wait before spawning a new worker.
  ///
  /// ```
  /// # use wapc_pool::HostPoolBuilder;
  /// # use std::time::Duration;
  /// let builder = HostPoolBuilder::new().max_wait(Duration::from_millis(100));
  /// ```
  ///
  pub fn max_wait(mut self, duration: Duration) -> Self {
    self.max_wait = duration;
    self
  }

  /// Builds a [HostPool] with the current configuration. Warning: this will panic if a factory function is not supplied.
  ///
  /// ```
  /// # use wapc_pool::HostPoolBuilder;
  /// # use wapc::WapcHost;
  /// # let bytes = std::fs::read("../../wasm/crates/wapc-guest-test/build/wapc_guest_test.wasm").unwrap();
  /// let engine = wasmtime_provider::WasmtimeEngineProvider::new(&bytes, None).unwrap();
  /// let pool = HostPoolBuilder::new()
  ///   .factory(move || {
  ///     let engine = engine.clone();
  ///     WapcHost::new(Box::new(engine), None).unwrap()
  ///   })
  ///   .build();
  /// ```
  ///
  pub fn build(mut self) -> HostPool {
    #[allow(clippy::expect_used)]
    let factory = self
      .factory
      .take()
      .expect("A waPC host pool must have a factory function.");
    HostPool::new(
      self.name.unwrap_or_else(|| "waPC host pool".to_owned()),
      factory,
      self.min_threads,
      self.max_threads,
      self.max_wait,
      self.max_idle,
    )
  }
}

#[cfg(test)]
mod tests {

  use std::time::{Duration, Instant};

  use tokio::join;
  use wapc::WebAssemblyEngineProvider;

  use super::*;

  #[test_log::test(tokio::test)]
  async fn test_basic() -> Result<()> {
    #[derive(Default)]
    struct Test {
      host: Option<Arc<wapc::ModuleState>>,
    }
    impl WebAssemblyEngineProvider for Test {
      fn init(
        &mut self,
        host: Arc<wapc::ModuleState>,
      ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.host = Some(host);
        Ok(())
      }

      fn call(
        &mut self,
        op_length: i32,
        msg_length: i32,
      ) -> std::result::Result<i32, Box<dyn std::error::Error + Send + Sync>> {
        println!("op len:{}", op_length);
        println!("msg len:{}", msg_length);
        std::thread::sleep(Duration::from_millis(100));
        let host = self.host.take().unwrap();
        host.set_guest_response(b"{}".to_vec());
        self.host.replace(host);
        Ok(1)
      }

      fn replace(&mut self, _bytes: &[u8]) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
      }
    }
    let pool = HostPoolBuilder::new()
      .name("test")
      .factory(move || WapcHost::new(Box::new(Test::default()), None).unwrap())
      .min_threads(5)
      .max_threads(5)
      .build();

    let now = Instant::now();
    let result = pool.call("test", b"hello world".to_vec()).await.unwrap();
    assert_eq!(result, b"{}");
    let _res = join!(
      pool.call("test", b"hello world".to_vec()),
      pool.call("test", b"hello world".to_vec()),
      pool.call("test", b"hello world".to_vec()),
      pool.call("test", b"hello world".to_vec()),
      pool.call("test", b"hello world".to_vec()),
      pool.call("test", b"hello world".to_vec()),
      pool.call("test", b"hello world".to_vec()),
      pool.call("test", b"hello world".to_vec()),
    );
    let duration = now.elapsed();
    println!("Took {}ms", duration.as_millis());
    assert!(duration.as_millis() < 600);

    Ok(())
  }

  #[test_log::test(tokio::test)]
  async fn test_elasticity() -> Result<()> {
    #[derive(Default)]
    struct Test {
      host: Option<Arc<wapc::ModuleState>>,
    }
    impl WebAssemblyEngineProvider for Test {
      fn init(
        &mut self,
        host: Arc<wapc::ModuleState>,
      ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.host = Some(host);
        Ok(())
      }

      fn call(&mut self, _: i32, _: i32) -> std::result::Result<i32, Box<dyn std::error::Error + Send + Sync>> {
        std::thread::sleep(Duration::from_millis(100));
        let host = self.host.take().unwrap();
        host.set_guest_response(b"{}".to_vec());
        self.host.replace(host);
        Ok(1)
      }

      fn replace(&mut self, _bytes: &[u8]) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
      }
    }
    let pool = HostPoolBuilder::new()
      .name("test")
      .factory(move || WapcHost::new(Box::new(Test::default()), None).unwrap())
      .min_threads(1)
      .max_threads(5)
      .max_wait(Duration::from_millis(10))
      .max_idle(Duration::from_secs(1))
      .build();
    assert_eq!(pool.num_active_workers(), 1);
    let _ = futures::future::join_all(vec![
      pool.call("test", b"hello world".to_vec()),
      pool.call("test", b"hello world".to_vec()),
      pool.call("test", b"hello world".to_vec()),
    ])
    .await;
    assert_eq!(pool.num_active_workers(), 2);
    let _ = futures::future::join_all(vec![
      pool.call("test", b"hello world".to_vec()),
      pool.call("test", b"hello world".to_vec()),
      pool.call("test", b"hello world".to_vec()),
      pool.call("test", b"hello world".to_vec()),
      pool.call("test", b"hello world".to_vec()),
      pool.call("test", b"hello world".to_vec()),
      pool.call("test", b"hello world".to_vec()),
      pool.call("test", b"hello world".to_vec()),
      pool.call("test", b"hello world".to_vec()),
    ])
    .await;
    assert_eq!(pool.num_active_workers(), 5);
    std::thread::sleep(Duration::from_millis(1500));
    assert_eq!(pool.num_active_workers(), 1);

    Ok(())
  }
}
