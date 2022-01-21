import { Context, Writer, BaseVisitor } from "@wapc/widl/ast";
import { expandType, isReference, functionName, isVoid } from "./helpers";
import { formatComment, shouldIncludeHandler } from "./utils";

export class HandlersVisitor extends BaseVisitor {
  constructor(writer: Writer) {
    super(writer);
  }

  visitOperation(context: Context): void {
    if (!shouldIncludeHandler(context)) {
      return;
    }
    if (context.config.handlerPreamble != true) {
      const className = context.config.handlersClassName || "Handlers";
      this.write(`#[cfg(feature = "guest")]
pub struct ${className} {}

#[cfg(feature = "guest")]
impl ${className} {\n`);
      context.config.handlerPreamble = true;
    }
    const operation = context.operation!;
    this.write(formatComment("    /// ", operation.description));
    const opName = operation.name.value;
    const fnName = functionName(operation.name.value);
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

    this.write(
      `pub fn register_${fnName}(f: fn(${paramTypes}) -> HandlerResult<${returnType}>) {
        *${fnName.toUpperCase()}.write().unwrap() = Some(f);
        register_function(&"${opName}", ${fnName}_wrapper);
      }`
    );
    super.triggerOperation(context);
  }

  visitAllOperationsAfter(context: Context): void {
    if (context.config.handlerPreamble == true) {
      this.write(`}\n\n`);
      delete context.config.handlerPreamble;
    }
    super.triggerAllOperationsAfter(context);
  }
}
