# `malgam-core`: Malgam Core Utilities

This crate contains core utilities used to manipulate Malgam markup and object notation. This library is used to:

- Compile markup to a static site (see `mal build`)
- Parse markup and object notation
- Transform markup to Markdown, and vice-versa
- Transform Malo to JSON, and vice-versa

The `mal` program, implemented in the `malgam-cli` crate, is mostly a wrapper over this library. It enables idiomatic manipulation of `.mal` and `.malo` files over the command-line, as well as a way for external programs to access the features listed above.

## The Malgam Language

**Malgam** is an ergonomic, extensible markup language inspired by Markdown. It is denoted by a `.mal` file extension.

### **2. Specification**

#### **Headings**

Headings are designated by 1-6 leading `=` characters.

Unlike in Markdown, these do not need to be followed by a whitespace character to be recognized. Also unlike Markdown, Malgam does not support the alternative syntax of headings with a line of `=` or `-` characters on the next line.

Though it is recommended to not include a trailing whitespace character, the inclusion of one **will not** break formatting but **will** be trimmed off in the rendered output.

| Malgam                | HTML                       | Rendered output          |
| --------------------- | -------------------------- | ------------------------ |
| =Heading level 1      | \<h1>Heading level 1\</h1> | <h1>Heading level 1</h1> |
| ==Heading level 2     | \<h2>Heading level 2\</h2> | <h2>Heading level 2</h2> |
| ===Heading level 3    | \<h3>Heading level 3\</h3> | <h3>Heading level 3</h3> |
| ====Heading level 4   | \<h4>Heading level 4\</h4> | <h4>Heading level 4</h4> |
| =====Heading level 5  | \<h5>Heading level 5\</h5> | <h4>Heading level 5</h5> |
| ======Heading level 6 | \<h6>Heading level 6\</h6> | <h5>Heading level 6</h6> |

If the formatter is run periodically, best practice is to leave headings lowercase. When `mal fmt` is run, the content is spellchecked and put into title case.

#### **Lists**

Malgam supports both unordered and ordered lists. 


### **3. Best Practices**

## Malgam Object Notation

**Malo** is an ergonomic, human-readable data serialization format derived from the Malgam. It is denoted by a `.malo` file extension.

While it shares similarities with JSON, it prioritizes ergonomic manual editing through features like flexible string quoting and trailing commas. There also exists a distinct syntax for lists and objects which respects Malgam macro syntax when such objects are used as macro arguments.

wrap is handled by alt-z

### **1. Data Types**

Malo supports six primary data types, mapped to the `MaloValue` enum:

| Type       | Description                                               | Example              |
| :--------- | :-------------------------------------------------------- | :------------------- |
| **Null**   | Represents an empty or non-existent value.                | `null`               |
| **Bool**   | A boolean logic value.                                    | `true`, `false`      |
| **Number** | IEEE 754 64-bit floating-point. Includes `inf` and `nan`. | `42`, `3.14`, `nan`  |
| **String** | UTF-8 text sequences. Supports single and double quotes.  | `"hello"`, `'world'` |
| **List**   | An ordered collection of values enclosed in braces.       | `{1, 2, 3}`          |
| **Object** | A collection of key-value pairs prefixed with a dot.      | `.{key=val}`         |

### **2. Common Pitfalls**

#### **Numbers**

- **Leading Digits:** Unlike some JSON parsers, Malo numbers **must** start with a digit.
- **Special Values:** Supports case-insensitive representations of infinity (`inf`, `+infinity`, `-inf`) and Not-a-Number (`nan`).

#### **Strings**

- **Quotes:** Both single quotes (`'`) and double quotes (`"`) are valid.
- **Multiline:** Malo supports pipe-prefixed (`|`) multiline strings, which strip the pipe character and preserve newlines. Such strings are terminated by a leading `;` on its own line.

#### **Lists and Objects**

- **Separators:** Commas (`,`) are used to separate items. Trailing commas are explicitly allowed and encouraged.

### **Object Keys**

- **Unquoted Keys:** Keys that satisfy `is_hgon_key_part()` (alphanumeric, underscores, and dashes) do not require quotes.
- **Quoted Keys:** If a key contains spaces or special characters, it must be quoted. Special characters are any .
- **Assignment:** Keys are mapped to values using the equals sign (`=`).

### **Delimiters**

- ***

## **3. Implementation Details**

### **The Tape Parser**

HGON uses a `Tape` abstraction for non-destructive reading of the byte stream.

- **`parse_any`**: The entry point for determining the type of the next token.
- **`consume`**: Used to skip whitespace (`is_hg_ws`) or collect specific character segments.

### **Formatting & Display**

The `HgonValue` implementation provides two ways to turn data back into strings:

1.  **Concise (`Display`):** Emits the smallest possible representation (e.g., `.{a:1,b:2,}`). Note that the standard `fmt` implementation uses `:` as a separator, while the parser looks for `=`.
2.  **Pretty Print (`to_pstring`):** Outputs a human-friendly version with 4-space indentations and quoted keys for clarity.

---

## **4. Examples**

### **Configuration Style**

```malo
.{
    project-name = "Malgam",
    version = 1.0,
    tags = {"compiler", "rust", "fast"},
    metadata = .{
        author = 'Dev',
        active = true,
    },
}
```

### **Multiline Strings**

```malo
.{
    description =
        | This is a
        | multiline string
        | in HGON,
}
```

---

## **5. Error Handling**

The parser returns an `HgonError` which provides the exact byte position (`pos`) of the failure:

- **`MissingValue`**: Found an empty space or comment where a value was expected.
- **`InvalidNumber`**: Failed to parse a string into an `f64`.
- **`IllegalCharacter`**: Encountered a byte that does not fit the expected grammar.
- **`MissingCloser`**: A collection (list/object) or string was opened but never closed.
