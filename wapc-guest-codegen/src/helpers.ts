import {
  Named,
  MapType,
  ListType,
  Optional,
  FieldDefinition,
  Type,
  Annotation,
  ValuedDefinition,
  OperationDefinition,
  ParameterDefinition,
  TypeDefinition,
  Kind,
} from "@wapc/widl/ast";
import { translations, primitives } from "./constant";
import { snakeCase } from "./utils";

/**
 * Takes an array of ValuedDefintions and returns a string based on supplied params.
 * @param sep seperator between name and type
 * @param joinOn string that each ValuedDefintion is joined on
 * @returns string of format <name> <sep> <type><joinOn>...
 */
export function mapVals(
  vd: ValuedDefinition[],
  sep: string,
  joinOn: string
): string {
  return vd
    .map(
      (vd) =>
        `${vd.name.value}${sep} ${expandType(
          vd.type,
          undefined,
          true,
          isReference(vd.annotations)
        )}`
    )
    .join(joinOn);
}

/**
 * Return default value for a FieldDefinition. Default value of objects are instantiated.
 * @param fieldDef FieldDefinition Node to get default value of
 */
export function defValue(fieldDef: FieldDefinition): string {
  const name = fieldDef.name.value;
  const type = fieldDef.type;
  if (fieldDef.default) {
    let returnVal = fieldDef.default.getValue();
    if (fieldDef.type.isKind(Kind.Named)) {
      returnVal =
        (fieldDef.type as Named).name.value == "string"
          ? strQuote(returnVal)
          : returnVal;
    }
    return returnVal;
  }

  switch (type.constructor) {
    case Optional:
      return "None";
    case ListType:
      return "Vec::new()";
    case MapType:
      return "MapType::new()";
    case Named:
      switch ((type as Named).name.value) {
        case "ID":
        case "string":
          return '""';
        case "bool":
          return "false";
        case "i8":
        case "u8":
        case "i16":
        case "u16":
        case "i32":
        case "u32":
        case "i64":
        case "u64":
        case "f32":
        case "f64":
          return "0";
        case "bytes":
          return "Vec::new()";
        default:
          return `${capitalize(name)}()`; // reference to something else
      }
  }
  return `???${expandType(
    type,
    undefined,
    false,
    isReference(fieldDef.annotations)
  )}???`;
}

export function defaultValueForType(type: Type, packageName?: string): string {
  switch (type.constructor) {
    case Optional:
      return "None";
    case ListType:
      return "Vec::new()";
    case MapType:
      return "MapType::new()";
    case Named:
      switch ((type as Named).name.value) {
        case "ID":
        case "string":
          return '"".to_string()';
        case "bool":
          return "false";
        case "i8":
        case "u8":
        case "i16":
        case "u16":
        case "i32":
        case "u32":
        case "i64":
        case "u64":
        case "f32":
        case "f64":
          return "0";
        case "bytes":
          return "Vec::new()";
        default:
          const prefix =
            packageName != undefined && packageName != ""
              ? packageName + "."
              : "";
          return `${prefix}${capitalize(
            (type as Named).name.value
          )}::default()`; // reference to something else
      }
  }
  return "???";
}

/**
 * returns string in quotes
 * @param s string to have quotes
 */
export const strQuote = (s: string) => {
  return `\"${s}\"`;
};

/**
 * returns string of the expanded type of a node
 * @param type the type node that is being expanded
 * @param useOptional if the type that is being expanded is optional
 * @param isReference if the type that is being expanded has a `@ref` annotation
 */
export const expandType = (
  type: Type,
  packageName: string | undefined,
  useOptional: boolean,
  isReference: boolean
): string => {
  switch (true) {
    case type.isKind(Kind.Named):
      if (isReference) {
        return "String";
      }
      var namedValue = (type as Named).name.value;
      const translation = translations.get(namedValue);
      if (translation != undefined) {
        return (namedValue = translation!);
      }
      if (isObject(type) && packageName != undefined && packageName != "") {
        return packageName + "." + namedValue;
      }
      return namedValue;
    case type.isKind(Kind.MapType):
      return `std::collections::HashMap<${expandType(
        (type as MapType).keyType,
        packageName,
        true,
        isReference
      )}, ${expandType(
        (type as MapType).valueType,
        packageName,
        true,
        isReference
      )}>`;
    case type.isKind(Kind.ListType):
      return `Vec<${expandType(
        (type as ListType).type,
        packageName,
        true,
        isReference
      )}>`;
    case type.isKind(Kind.Optional):
      const nestedType = (type as Optional).type;
      let expanded = expandType(nestedType, packageName, true, isReference);
      if (useOptional) {
        return `Option<${expanded}>`;
      }
      return expanded;
    default:
      return "unknown";
  }
};

/**
 * Determines if a node is a void node
 * @param t Node that is a Type node
 */
export function isVoid(t: Type): boolean {
  if (t.isKind(Kind.Named)) {
    return (t as Named).name.value == "void";
  }
  return false;
}

/**
 * Determines if Type Node is a Named node and if its type is not one of the base translation types.
 * @param t Node that is a Type node
 */
export function isObject(t: Type): boolean {
  if (t.isKind(Kind.Named)) {
    return !primitives.has((t as Named).name.value);
  }
  return false;
}

/**
 * Determines if one of the annotations provided is a reference
 * @param annotations array of Annotations
 */
export function isReference(annotations: Annotation[]): boolean {
  for (let annotation of annotations) {
    if (
      annotation.name.value == "ref" ||
      annotation.name.value == "reference"
    ) {
      return true;
    }
  }
  return false;
}

/**
 * Capitalizes a given string
 * @param str string to be capitlized
 * @returns string with first character capitalized. If empty string returns empty string.
 */
export function capitalize(str: string): string {
  if (str.length == 0) return str;
  if (str.length == 1) return str[0].toUpperCase();
  return str[0].toUpperCase() + str.slice(1);
}

export function functionName(str: string): string {
  return snakeCase(str);
}

export function fieldName(str: string): string {
  return snakeCase(str);
}

/**
 * Given an array of OperationDefintion returns them as functions with their arguments
 * @param ops
 */
export function opsAsFns(ops: OperationDefinition[]): string {
  return ops
    .map((op) => {
      return `func ${op.name.value}(${mapArgs(op.parameters)}) ${expandType(
        op.type,
        undefined,
        true,
        isReference(op.annotations)
      )} {\n}`;
    })
    .join("\n");
}

/**
 * returns string of args mapped to their type
 * @param args InputValueDefintion array which is an array of the arguments
 */
export function mapArgs(
  args: ParameterDefinition[],
  template: boolean = false
): string {
  return args
    .map((arg) => {
      return mapArg(arg, template);
    })
    .join(", ");
}

export function mapArg(
  arg: ParameterDefinition,
  template: boolean = false
): string {
  return (
    (template ? "_" : "") +
    `${arg.name.value}: ${expandType(
      arg.type,
      undefined,
      true,
      isReference(arg.annotations)
    )}`
  );
}

/**
 * returns if a widl type is a node
 * @param o TypeDefinition which correlates to a widl Type
 */
export function isNode(o: TypeDefinition): boolean {
  for (const field of o.fields) {
    if (field.name.value.toLowerCase() == "id") {
      return true;
    }
  }
  return false;
}

export function varAccessArg(
  variable: string,
  args: ParameterDefinition[]
): string {
  return args
    .map((arg) => {
      return `${variable}.${snakeCase(arg.name.value)}`;
    })
    .join(", ");
}
