# waPC code generator for Rust guests

This library provides the code generation for waPC guests written in Rust. It works in tandem with [`wapc/widl-js`](https://github.com/wapc/widl-js) to generate waPC guest code.

## Installation

```sh
$ npm install @wapc/codegen-rust-guest
```

## Usage (node)

```js
const widl = require("@wapc/widl");
const ast = require("@wapc/widl/ast");
const rust = require("@wapc/codegen-rust-guest");

const schema = `
namespace "mandelbrot"

interface {
  update(width: u32, height: u32, limit: u32): [u16]
}`;

const context = new ast.Context({});

const doc = widl.parse(schema, { noLocation: true });
const writer = new ast.Writer();
const visitor = new rust.ScaffoldVisitor(writer);
doc.accept(context, visitor);
let source = writer.string();

console.log(source);
```

## License

[Apache License 2.0](https://choosealicense.com/licenses/apache-2.0/)
