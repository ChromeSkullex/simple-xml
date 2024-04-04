## Note
Hello! This was a project made by [Freja Roberts](https://crates.io/users/ten3roberts) and want to first give credit for them. I'm still slowly updating this project because it's super intuitive but just needs updating. 

# Szl Simple XML
Szl Simple xml is a small crate for reading, parsing and storing xml as an extension from [simple-xml](https://crates.io/crates/simple-xml). This extension adds getting mutable nodes

## Usage
Example parsing:

``` rust

let note =
    szl_simple_xml::from_file("./examples/note.xml").expect("Failed to parse simple_xml");

let to = &note["to"][0];
let from = &note["from"][0];
let heading = &note.get_nodes("heading").expect("Missing heading")[0];
let body = &note["body"][0];
let lang = note
    .get_attribute("lang")
    .expect("Failed to get attribute lang");
```

For additional examples, please see: [Docs](https://docs.rs/szl-simple-xml/0.1.1/szl_simple_xml/)