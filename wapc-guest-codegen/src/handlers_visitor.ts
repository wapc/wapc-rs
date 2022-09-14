import { Context, Writer, BaseVisitor } from "@apexlang/core/model";
import { functionName } from "./helpers";
import { formatComment, shouldIncludeHandler } from "./utils";
import * as utils from "@apexlang/codegen/utils";
import { utils as rustUtils } from "@apexlang/codegen/rust";

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
      this.write(`
#[cfg(feature = "guest")]
pub struct ${className} {}

#[cfg(feature = "guest")]
impl ${className} {
`);
      context.config.handlerPreamble = true;
    }
    const operation = context.operation!;
    this.write(formatComment("    /// ", operation.description));
    const opName = operation.name;
    const fnName = functionName(operation.name);
    const paramTypes = operation.parameters
      .map((param) =>
        rustUtils.types.apexToRustType(param.type, context.config)
      )
      .join(",");
    const returnType = utils.isVoid(operation)
      ? "()"
      : rustUtils.types.apexToRustType(operation.type, context.config);

    this.write(`
pub fn register_${fnName}(f: fn(${paramTypes}) -> HandlerResult<${returnType}>) {
  *${fnName.toUpperCase()}.write().unwrap() = Some(f);
  register_function("${opName}", ${fnName}_wrapper);
}`);
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
