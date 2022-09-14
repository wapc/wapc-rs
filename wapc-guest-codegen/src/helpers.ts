import { Parameter, ObjectMap } from "@apexlang/core/model.js";
import { snakeCase } from "./utils/index.js";
import { utils as rustUtils } from "@apexlang/codegen/rust";

export function functionName(str: string): string {
  return rustUtils.rustify(str);
}

export function fieldName(str: string): string {
  return rustUtils.rustify(str);
}

/**
 * Returns string of args mapped to their type
 */
export function mapArgs(
  args: Parameter[],
  config: ObjectMap<any>,
  template: boolean = false
): string {
  return args
    .map((arg) => {
      return mapArg(arg, config, template);
    })
    .join(", ");
}

export function mapArg(
  arg: Parameter,
  config: ObjectMap<any>,
  template: boolean = false
): string {
  return (
    (template ? "_" : "") +
    `${arg.name}: ${rustUtils.types.apexToRustType(arg.type, config)}`
  );
}

export function varAccessArg(variable: string, args: Parameter[]): string {
  return args
    .map((arg) => {
      return `${variable}.${snakeCase(arg.name)}`;
    })
    .join(", ");
}
