import { Context, Writer, BaseVisitor } from "@apexlang/core/model";
import { fieldName, functionName } from "./helpers";
import { formatComment, shouldIncludeHostCall } from "./utils";
import * as utils from "@apexlang/codegen/utils";
import { utils as rustUtils } from "@apexlang/codegen/rust";

export class HostVisitor extends BaseVisitor {
  constructor(writer: Writer) {
    super(writer);
  }

  visitOperation(context: Context): void {
    if (!shouldIncludeHostCall(context)) {
      return;
    }
    if (context.config.hostPreamble != true) {
      const className = context.config.hostClassName || "Host";
      this.write(`
#[cfg(feature = "guest")]
pub struct ${className} {
    binding: String,
}

#[cfg(feature = "guest")]
impl Default for ${className} {
    fn default() -> Self {
      ${className} {
            binding: "default".to_string(),
        }
    }
}

/// Creates a named host binding
#[cfg(feature = "guest")]
pub fn host(binding: &str) -> ${className} {
  ${className} {
        binding: binding.to_string(),
    }
}

/// Creates the default host binding
#[cfg(feature = "guest")]
pub fn default() -> ${className} {
  ${className}::default()
}

#[cfg(feature = "guest")]
impl ${className} {`);
      context.config.hostPreamble = true;
    }
    const operation = context.operation!;
    this.write(formatComment("  /// ", operation.description));
    this.write(`pub fn ${functionName(operation.name)}(&self`);
    operation.parameters.map((param, index) => {
      this.write(
        `, ${fieldName(param.name)}: ${rustUtils.types.apexToRustType(
          param.type,
          context.config
        )}`
      );
    });
    this.write(`) `);
    const retVoid = utils.isVoid(operation.type);
    if (!retVoid) {
      this.write(
        `-> HandlerResult<${rustUtils.types.apexToRustType(
          operation.type,
          context.config
        )}>`
      );
    } else {
      this.write(`-> HandlerResult<()>`);
    }
    this.write(` {\n`);

    if (operation.parameters.length == 0) {
      this.write(`
host_call(
  &self.binding,
  "${context.namespace.name}",
  "${operation.name}",
  &vec![],
)
`);
    } else if (operation.isUnary()) {
      this.write(`
host_call(
  &self.binding,
  "${context.namespace.name}",
  "${operation.name}",
  &messagepack::serialize(${operation.unaryOp().name})?,
)
`);
    } else {
      let params = operation.parameters.map((param) => fieldName(param.name));
      this.write(`
let input_args = ${rustUtils.rustifyCaps(operation.name)}Args{
  ${params.join(",")}
};`);
      this.write(`
host_call(
  &self.binding,
  "${context.namespace.name}",
  "${operation.name}",
  &messagepack::serialize(input_args)?,
)
`);
    }
    if (!retVoid) {
      this.write(`
        .map(|vec| {
        messagepack::deserialize::<${rustUtils.types.apexToRustType(
          operation.type,
          context.config
        )}>(vec.as_ref()).unwrap()
      })\n`);
    } else {
      this.write(`.map(|_vec| ())\n`);
    }
    this.write(`}\n`);
    super.triggerOperation(context);
  }

  visitAllOperationsAfter(context: Context): void {
    if (context.config.hostPreamble == true) {
      this.write(`}\n\n`);
      delete context.config.hostPreamble;
    }
    super.triggerAllOperationsAfter(context);
  }
}
