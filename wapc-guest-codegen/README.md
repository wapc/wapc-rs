# WIDL Code Generation

This library works in tandem with [`wapc/widl-js`](https://github.com/wapc/widl-js) to generate waPC
module code in Rust, AssemblyScript, and TinyGo.

## Installation

```sh
$ npm install @wapc/widl
$ npm install @wapc/widl-codegen
```

## Usage (node)

```js
const widl = require("@wapc/widl");
const ast = require("@wapc/widl/ast");
const assemblyscript = require("@wapc/widl-codegen/assemblyscript");

const schema = `
namespace "mandelbrot"

interface {
  update(width: u32, height: u32, limit: u32): [u16]
}`;

const context = new ast.Context({});

const doc = widl.parse(schema, { noLocation: true });
const writer = new ast.Writer();
const visitor = new assemblyscript.ScaffoldVisitor(writer);
doc.accept(context, visitor);
let source = writer.string();

console.log(source);
```

## Usage (browser)

```html
<html>
  <head>
    <script src="https://unpkg.com/prettier@2.2.1/standalone.js"></script>
    <script src="https://unpkg.com/prettier@2.2.1/parser-typescript.js"></script>
    <script src="https://unpkg.com/@wapc/widl/dist/standalone.min.js"></script>
    <script src="https://unpkg.com/@wapc/widl-codegen/dist/standalone.min.js"></script>
  </head>
  <body>
    <pre><code id="generated"></code></pre>
  </body>
  <script type="text/javascript">

const doc = widl.parse(`namespace "mandelbrot"

interface {
    update(width: u32, height: u32, limit: u32): [u16]
}`);
const context = new widl.ast.Context({});

const writer = new widl.ast.Writer();
const visitor = new widl.codegen.assemblyscript.ScaffoldVisitor(writer);
doc.accept(context, visitor);
let source = writer.string();

source = prettier.format(source, {
  parser: "typescript",
  plugins: prettierPlugins,
});

document.getElementById("generated").innerHTML = source;

  </script>
</html>
```

## License

[Apache License 2.0](https://choosealicense.com/licenses/apache-2.0/)