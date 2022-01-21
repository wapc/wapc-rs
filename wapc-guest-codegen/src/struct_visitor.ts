import { Context, Writer, BaseVisitor } from "@wapc/widl/ast";
import { expandType, fieldName, isReference } from "./helpers";
import { formatComment } from "./utils";

export class StructVisitor extends BaseVisitor {
  constructor(writer: Writer) {
    super(writer);
  }

  visitTypeBefore(context: Context): void {
    super.triggerTypeBefore(context);
    this.write(formatComment("/// ", context.type!.description));
    this
      .write(`#[derive(Debug, PartialEq, Demessagepack::serialize, Serialize, Default, Clone)]
pub struct ${context.type!.name.value} {\n`);
  }

  visitTypeField(context: Context): void {
    const field = context.field!;
    const expandedType = expandType(
      field.type!,
      undefined,
      true,
      isReference(field.annotations)
    );
    this.write(formatComment("  /// ", field.description));
    if (expandedType.indexOf("Vec<u8>") != -1) {
      this.write(`#[serde(with = "serde_bytes")]\n`);
    }
    this.write(
      `\t#[serde(rename = "${field.name.value}")]
      \tpub ${fieldName(field.name.value)}: ${expandedType},\n`
    );
    super.triggerTypeField(context);
  }

  visitTypeAfter(context: Context): void {
    this.write(`}\n\n`);
  }
}
