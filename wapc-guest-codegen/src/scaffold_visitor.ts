import { Context, Writer, BaseVisitor } from "@apexlang/core/model";
import { shouldIncludeHandler } from "./utils";
import { functionName, mapArgs } from "./helpers";
import utils from "@apexlang/codegen/utils";
import { utils as rustUtils } from "@apexlang/codegen/rust";

export class ScaffoldVisitor extends BaseVisitor {
  constructor(writer: Writer) {
    super(writer);
  }

  visitContextBefore(context: Context): void {
    const useName = context.config["use"] || "generated";
    super.visitContextBefore(context);
    this.write(`mod ${useName};
use wapc_guest::prelude::*;
pub use ${useName}::*;\n\n`);
  }

  visitAllOperationsBefore(context: Context): void {
    const registration = new HandlerRegistrationVisitor(this.writer);
    context.accept(context, registration);
  }

  visitOperation(context: Context): void {
    if (!shouldIncludeHandler(context)) {
      return;
    }
    const operation = context.operation!;
    this.write(`\n`);
    this.write(
      `fn ${functionName(operation.name)}(${mapArgs(
        operation.parameters,
        context.config,
        true
      )}) -> HandlerResult<`
    );
    if (!utils.isVoid(operation.type)) {
      this.write(
        rustUtils.types.apexToRustType(operation.type, context.config)
      );
    } else {
      this.write(`()`);
    }
    this.write(`> {\n`);
    if (!utils.isVoid(operation.type)) {
      const dv = rustUtils.types.defaultValue(operation.type, context.config);
      this.write(`    Ok(${dv})`);
    } else {
      this.write(`    Ok(())`);
    }
    this.write(` // TODO: Provide implementation.\n`);
    this.write(`}\n`);
  }
}

class HandlerRegistrationVisitor extends BaseVisitor {
  constructor(writer: Writer) {
    super(writer);
  }

  visitAllOperationsBefore(context: Context): void {
    this.write(`#[no_mangle]
pub fn wapc_init() {\n`);
  }

  visitOperation(context: Context): void {
    if (!shouldIncludeHandler(context)) {
      return;
    }
    const operation = context.operation!;
    this.write(
      `    Handlers::register_${functionName(operation.name)}(${functionName(
        operation.name
      )});\n`
    );
  }

  visitAllOperationsAfter(context: Context): void {
    this.write(`}\n`);
  }
}
