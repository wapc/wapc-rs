import { Context, Writer, BaseVisitor } from "@wapc/widl/ast";
import {
  expandType,
  isReference,
  capitalize,
  isVoid,
  varAccessArg,
  functionName,
} from "./helpers";
import { shouldIncludeHandler } from "./utils";

export class WrapperVarsVisitor extends BaseVisitor {
  constructor(writer: Writer) {
    super(writer);
  }

  visitOperation(context: Context): void {
    if (!shouldIncludeHandler(context)) {
      return;
    }
    const operation = context.operation!;
    const fnName = functionName(operation.name.value).toUpperCase();
    const paramTypes = operation.parameters
      .map((param) =>
        expandType(param.type, undefined, true, isReference(param.annotations))
      )
      .join(",");
    const returnType = isVoid(operation.type)
      ? "()"
      : expandType(
          operation.type,
          undefined,
          true,
          isReference(operation.annotations)
        );

    this.write(`
        #[cfg(feature = "guest")]
        static ${fnName}: once_cell::sync::Lazy<std::sync::RwLock<Option<fn(${paramTypes}) -> HandlerResult<${returnType}>>>> =
          once_cell::sync::Lazy::new(|| std::sync::RwLock::new(None));
        `);
  }

  visitAllOperationsAfter(context: Context): void {
    if (context.config.handlerPreamble == true) {
      this.write(`}\n\n`);
    }
    super.triggerAllOperationsAfter(context);
  }
}

export class WrapperFuncsVisitor extends BaseVisitor {
  constructor(writer: Writer) {
    super(writer);
  }

  visitOperation(context: Context): void {
    if (!shouldIncludeHandler(context)) {
      return;
    }
    const operation = context.operation!;
    const fnName = functionName(operation.name.value);
    let inputType = "",
      inputArgs = "";
    if (operation.isUnary()) {
      inputType = expandType(
        operation.unaryOp().type,
        undefined,
        false,
        isReference(operation.annotations)
      );
      inputArgs = "input";
    } else {
      inputType = `${capitalize(operation.name.value)}Args`;
      inputArgs = varAccessArg("input", operation.parameters);
    }

    this.write(`
      #[cfg(feature = "guest")]
      fn ${fnName}_wrapper(input_payload: &[u8]) -> CallResult {
        let input = messagepack::deserialize::<${inputType}>(input_payload)?;
        let lock = ${fnName.toUpperCase()}.read().unwrap().unwrap();
        let result = lock(${inputArgs})?;
        Ok(messagepack::serialize(result)?)
      }`);
  }

  visitWrapperBeforeReturn(context: Context): void {
    this.triggerCallbacks(context, "WrapperBeforeReturn");
  }
}
