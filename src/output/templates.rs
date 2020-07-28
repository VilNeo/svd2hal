use lazy_static::lazy_static;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use tera::{Context, Result, Tera, Value};

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        fn hex_filter(value: &Value, _arguments: &HashMap<String, Value>) -> tera::Result<Value> {
            match value {
                Value::Null => Ok(Value::Null),
                Value::Bool(b) => Ok(if *b {
                    Value::String("0x1".to_string())
                } else {
                    Value::String("0x0".to_string())
                }),
                Value::Number(n) => Ok(Value::String(format!("{:#X}", n.as_u64().unwrap()))),
                Value::String(s) => Err(tera::Error::msg(format!(
                    "String to hex not supported for {}",
                    s
                ))),
                Value::Array(s) => Err(tera::Error::msg("Array to hex not supported.")),
                Value::Object(s) => Err(tera::Error::msg("Object to hex not supported.")),
            }
        }
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            (CARGO_TOML_TEMPLATE, CARGO_TOML_TEMPLATE_CONTENT),
            (SRC_LIB_RS_TEMPLATE, SRC_LIB_RS_TEMPLATE_CONTENT),
            (PERIPHERALS_TEMPLATE, PERIPHERALS_TEMPLATE_CONTENT),
            (LINKER_TEMPLATE, LINKER_TEMPLATE_CONTENT),
            (REG_DEF_MACROS, REG_DEF_MACROS_CONTENT),
        ])
        .unwrap();
        tera.autoescape_on(vec![]);
        tera.register_filter("hex", hex_filter);
        tera
    };
}

pub static CARGO_TOML_TEMPLATE: &'static str = "cargo.toml";
static CARGO_TOML_TEMPLATE_CONTENT: &'static str = "\
[package]
name = \"{{project_name}}\"
version = \"0.1.0\"
authors = [\"Alexander Huymayer <alex@peiran.de>\"]
edition = \"2018\"
";

static REG_DEF_MACROS: &'static str = "reg_def_macros.rs";
static REG_DEF_MACROS_CONTENT: &'static str = "\
{%- macro fieldType(fieldType) -%}
        {%-if fieldType.raw-%}
            {{fieldType.raw}}
        {%-else-%}
            enum:
            {%-if fieldType.enum.content-%}
                {{fieldType.enum.content.name}}
            {%-else-%}
                {{fieldType.enum.derived.name}}
            {%-endif-%}
        {%-endif-%}
{%- endmacro input -%}";

pub static SRC_LIB_RS_TEMPLATE: &'static str = "src_lib.rs";
static SRC_LIB_RS_TEMPLATE_CONTENT: &'static str = "\
#![no_std]

#[macro_use]
mod macros;
mod hal;
mod peripherals;

pub use hal::*;
";

pub static PERIPHERALS_TEMPLATE: &'static str = "peripherals.rs";
static PERIPHERALS_TEMPLATE_CONTENT: &'static str = "
{%- import \"reg_def_macros.rs\" as macros -%}
#![allow(non_snake_case)]
{%- for peripheral in peripherals -%}
{%-if peripheral.content%}
mod {{peripheral.content.name}}{
{%- for register in peripheral.content.registers %}
    create_reg! { {{peripheral.content.name}}::{{register.name}}(u32) =>
        {%-if register.readWriteFields%}
        RW{
            {% for field in register.readWriteFields -%}
            {{field.name}}({{field.mask | hex}}, {{ macros::fieldType(fieldType=field.fieldType) }}),
            {%- endfor %}
        }
        {%-endif-%}
        {%-if register.readFields%}
        R{
            {% for field in register.readFields -%}
            {{field.name}}({{field.mask | hex}}, {{ macros::fieldType(fieldType=field.fieldType) }}),
            {%- endfor %}
        }
        {%-endif-%}
        {%-if register.writeFields%}
        W{
            {% for field in register.writeFields -%}
            {{field.name}}({{field.mask | hex}}, {{ macros::fieldType(fieldType=field.fieldType) }}),
            {%- endfor %}
        }
        {%-endif%}
    }
{%- endfor %}
}
{%-elif peripheral.derived%}
//Derived peripherals not supported yet
{%-endif-%}
{%-endfor%}";

pub static PERIPHERAL_CONTENT_TEMPLATE: &'static str = "peripheral_content.rs";
static PERIPHERAL_CONTENT_TEMPLATE_CONTENT: &'static str = "\
extern \"C\" \\{
    #[no_mangle]
    pub static mut {content.name}: {content.name}_struct;
}
#[repr(C)]
#[no_mangle]
pub struct {content.name}_struct \\{\
    {{ for register in content.registers }}
    pub {register.name}: u{register.size},\
    {{ endfor }}
}
";

pub static PERIPHERAL_LINK_TEMPLATE: &'static str = "peripheral_link.rs";
static PERIPHERAL_LINK_TEMPLATE_CONTENT: &'static str = "\
#[allow(non_snake_case)]
extern \"C\" \\{
    #[no_mangle]
    pub static mut {link.name}: {link.derivedFrom}_struct;
}
";

pub static PERIPHERAL_MOD_TEMPLATE: &'static str = "peripheral_mod.rs";
static PERIPHERAL_MOD_TEMPLATE_CONTENT: &'static str = "\
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[macro_use]
pub mod macros;
pub mod types;
";

pub static LINKER_TEMPLATE: &'static str = "peripheral.x";
static LINKER_TEMPLATE_CONTENT: &'static str = "\
SECTIONS { \
{% for peripheral in peripherals %}
    {% if peripheral.content %}{{peripheral.content.name}} = {{peripheral.content.baseAddress}};\
    {% elif peripheral.derived %}{{peripheral.derived.name}} = {{peripheral.derived.baseAddress}};\
    {%endif%}\
{% endfor %}
}";

pub static HAL_MOD_RS_TEMPLATE: &'static str = "hal_mod.rs";
static HAL_MOD_RS_TEMPLATE_CONTENT: &'static str = "\
{{ for definition in definitions }}\
pub mod {definition.name};
{{ endfor }}";

pub static HAL_TEMPLATE: &'static str = "hal.rs";
static HAL_TEMPLATE_CONTENT: &'static str = "\
#![allow(dead_code)]
{{ for entity in halEntities }}
pub mod {entity.name | snake} \\{
{{call hal_mod.rs.entity_types with entity}}
{{call hal_mod.rs.entity_regs with entity}}


    pub fn set() -> Writer \\{
        Writer \\{
            _apply_on_drop: true,
{{ for peripheral in entity.peripherals }}\
{{ for register in peripheral.registers }}
            {peripheral.name | snake}_{register.name | snake}_reg: {peripheral.name}::{register.name}::new(),\
{{endfor}}\
{{endfor}}
        }
    }
    pub fn read() -> Reader \\{
        Reader \\{
{{ for peripheral in entity.peripherals }}\
{{ for register in peripheral.registers }}
            {peripheral.name | snake}_{register.name | snake}_reg: None,\
{{endfor}}\
{{endfor}}
        }
    }

    pub struct Writer \\{
        _apply_on_drop: bool,
{{ for peripheral in entity.peripherals }}\
{{ for register in peripheral.registers }}
        {peripheral.name | snake}_{register.name | snake}_reg: {peripheral.name}::{register.name}::Writer,\
{{endfor}}\
{{endfor}}
    }

    pub struct Reader \\{
{{ for peripheral in entity.peripherals }}\
{{ for register in peripheral.registers }}
        {peripheral.name | snake}_{register.name | snake}_reg: Option<{peripheral.name}::{register.name}::Reader>,\
{{endfor}}
{{endfor}}
    }

    impl Reader \\{
{{ for peripheral in entity.peripherals }}\
{{ for register in peripheral.registers }}
        fn update_{peripheral.name | snake}_{register.name | snake}_reg(&mut self) \\{
            self.{peripheral.name | snake}_{register.name | snake}_reg = Some({peripheral.name}::{register.name}::read());\
        }
//ToDo: Move to call...
{{ for field in register.readFields }}\
        {{if field.visible}}
        pub fn {field.name | snake}(&mut self) -> {field.fieldType} \\{
            if self.{peripheral.name | snake}_{register.name | snake}_reg.is_none() \\{
                self.update_{peripheral.name | snake}_{register.name | snake}_reg();
            }
            self.{peripheral.name | snake}_{register.name | snake}_reg.as_ref().unwrap().{field.svdName}()
        }\
        {{endif}}\
{{endfor}}\
{{ for field in register.readWriteFields }}\
        {{if field.visible}}
        pub fn {field.name | snake}(&mut self) -> {field.fieldType} \\{
            if self.{peripheral.name | snake}_{register.name | snake}_reg.is_none() \\{
                self.update_{peripheral.name | snake}_{register.name | snake}_reg();
            }
            self.{peripheral.name | snake}_{register.name | snake}_reg.as_ref().unwrap().{field.svdName}()
        }\
        {{endif}}\
{{endfor}}\
{{endfor}}
{{endfor}}
    }

    impl Writer \\{
        pub fn write(&self) \\{
{{ for peripheral in entity.peripherals }}\
{{ for register in peripheral.registers }}
            self.{peripheral.name | snake}_{register.name | snake}_reg.write();\
{{endfor}}
{{endfor}}
        }
{{ for peripheral in entity.peripherals }}\
{{ for register in peripheral.registers }}\
{{ for field in register.writeFields }}\
        {{if field.visible}}
        pub fn {field.name | snake}(&mut self, value: {field.fieldType}) -> &mut Self \\{
            self.{peripheral.name | snake}_{register.name | snake}_reg.{field.svdName}(value);
            self
        }\
        {{endif}}\
{{endfor}}\
{{ for field in register.readWriteFields }}\
        {{if field.visible}}
        pub fn {field.name | snake}(&mut self, value: {field.fieldType}) -> &mut Self \\{
            self.{peripheral.name | snake}_{register.name | snake}_reg.{field.svdName}(value);
            self
        }\
        {{endif}}\
{{endfor}}\
{{endfor}}
{{endfor}}
    }

    impl Drop for Writer \\{
        fn drop(&mut self) \\{
            if !self._apply_on_drop \\{
                return;
            }
            self.write();
        }
    }
}
{{endfor}}
";

pub static HAL_MOD_RS_CONFIG_TEMPLATE: &'static str = "hal_mod.rs.config";
static HAL_MOD_RS_CONFIG_TEMPLATE_CONTENT: &'static str = "\
{{ for entity in @root }}
    pub fn {entity.name | snake}() -> {entity.name | pascal}::{entity.name | pascal} \\{
        {entity.name | pascal}::{entity.name | pascal}::new()
    }
    pub fn read_{entity.name | snake}() -> {entity.name | pascal}::{entity.name | pascal} \\{
        {entity.name | pascal}::{entity.name | pascal}::read()
    }\
{{ endfor }}";

pub static HAL_MOD_RS_ENTITY_STRUCT_TEMPLATE: &'static str = "hal_mod.rs.entity_struct";
static HAL_MOD_RS_ENTITY_STRUCT_TEMPLATE_CONTENT: &'static str =
    "    pub struct {name | pascal} \\{
        _apply_on_drop: bool,\
{{ for peripheral in peripherals }}\
{{ for register in peripheral.registers }}\
{{ for field in register.readFields }}\
        {{if field.visible}}
        pub _{field.name | snake}: Option<{field.fieldType}>,\
        {{endif}}\
{{ endfor }}\
{{ for field in register.readWriteFields }}\
        {{if field.visible}}
        pub _{field.name | snake}: Option<{field.fieldType}>,\
        {{endif}}\
{{ endfor }}\
{{ endfor }}\
{{ endfor }}
    }
";

pub static HAL_MOD_RS_ENTITY_REG_TEMPLATE: &'static str = "hal_mod.rs.entity_regs";
static HAL_MOD_RS_ENTITY_REG_TEMPLATE_CONTENT: &'static str = "\
{{ for peripheral in peripherals }}
    #[allow(non_snake_case)]
    mod {peripheral.name}\\{\
{{ for register in peripheral.registers }}
        create_reg! \\{{peripheral.name}::{register.name}(u32) =>\
            {{if register.readWriteFields}}
            RW\\{
                {{ for field in register.readWriteFields }}\
                {field.svdName}({field.mask}{field.fieldTypeDelimiter}{field.fieldType}),\
                {{ endfor }}
            }\
            {{endif}}\
            {{if register.readFields}}
            R\\{
                {{ for field in register.readFields }}\
                {field.svdName}({field.mask}{field.fieldTypeDelimiter}{field.fieldType}),\
                {{ endfor }}
            }\
            {{endif}}\
            {{if register.writeFields}}
            W\\{
                {{ for field in register.writeFields }}\
                {field.svdName}({field.mask}{field.fieldTypeDelimiter}{field.fieldType}),\
                {{ endfor }}
            }\
            {{endif}}
        }\
{{ endfor }}
    }\
{{ endfor }}";

pub static HAL_MOD_RS_ENTITY_IMPL_TEMPLATE: &'static str = "hal_mod.rs.entity_impl";
static HAL_MOD_RS_ENTITY_IMPL_TEMPLATE_CONTENT: &'static str =
"    impl {name | pascal} \\{
        pub fn new() -> {name | pascal} \\{
            {name | pascal} \\{
                _apply_on_drop: true,\
{{ for peripheral in peripherals }}\
{{ for register in peripheral.registers }}\
{{ for field in register.readFields }}\
                {{if field.visible}}
                _{field.name | snake}: None,\
                {{endif}}\
{{ endfor }}\
{{ for field in register.readWriteFields }}\
                {{if field.visible}}
                _{field.name | snake}: None,\
                {{endif}}\
{{ endfor }}\
{{ endfor }}\
{{ endfor }}
            }
        }
        pub fn read() -> {name | pascal} \\{
{{ for peripheral in peripherals }}\
{{ for register in peripheral.registers }}
            let {peripheral.name | snake}_{register.name | snake}_config = {peripheral.name}::{register.name}::read();\
{{ endfor }}\
{{ endfor }}
            {name | pascal} \\{
                _apply_on_drop: false,\
{{ for peripheral in peripherals }}\
{{ for register in peripheral.registers }}\
{{ for field in register.readFields }}\
                {{if field.visible}}
                _{field.name | snake}: {peripheral.name | snake}_{register.name | snake}_config.{field.svdName}.ok(),\
                {{endif}}\
{{ endfor }}\
{{ for field in register.readWriteFields }}\
                {{if field.visible}}
                _{field.name | snake}: {peripheral.name | snake}_{register.name | snake}_config.{field.svdName}.ok(),\
                {{endif}}\
{{ endfor }}\
{{ endfor }}\
{{ endfor }}
            }
        }
{{ for peripheral in peripherals }}\
{{ for register in peripheral.registers }}\
{{ for field in register.writeFields }}\
        {{if field.visible}}
        pub fn {field.name | snake}(&mut self, {field.name | snake}: {field.fieldType}) -> &mut Self \\{
            self._{field.name | snake} = Some({field.name | snake});
            self
        }\
        {{endif}}\
{{ endfor }}\
{{ for field in register.readWriteFields }}\
        {{if field.visible}}
        pub fn {field.name | snake}(&mut self, {field.name | snake}: {field.fieldType}) -> &mut Self \\{
            self._{field.name | snake} = Some({field.name | snake});
            self
        }\
        {{endif}}\
{{ endfor }}\
{{ endfor }}\
{{ endfor }}
    }";

pub static HAL_MOD_RS_ENTITY_TYPES_TEMPLATE: &'static str = "hal_mod.rs.entity_types";
static HAL_MOD_RS_ENTITY_TYPES_TEMPLATE_CONTENT: &'static str = "\
{{ for type in aggregatedTypes }}
    #[allow(non_camel_case_types)]
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub enum {type.name} \\{\
        {{for value in type.values}}\
        {{ if value }}
        {value} = { @index },\
        {{ endif }}\
        {{ endfor }}
    }
    impl {type.name} \\{
        pub fn from(x: u32) -> {type.name} \\{
            match x \\{\
                {{for value in type.values}}\
                {{ if value }}
                { @index } => {type.name}::{value},\
                {{ endif }}\
                {{ endfor }}
                _ => panic!(\"Error while reading type {type.name}. Got value \\{}\",x),
            }
        }
    }\
{{ endfor }}
";

pub static HAL_MOD_RS_ENTITY_DROP_TEMPLATE: &'static str = "hal_mod.rs.entity_drop";
static HAL_MOD_RS_ENTITY_DROP_TEMPLATE_CONTENT: &'static str = "    impl Drop for {name} \\{
        fn drop(&mut self) \\{
            if !self._apply_on_drop \\{
                return;
            }\
{{if drop}}
            {drop | unescaped}
{{else}}\
{{ for peripheral in peripherals }}\
{{ for register in peripheral.registers }}
            {peripheral.name}::{register.name}::new()
            {{ for field in register.readWriteFields }}\
            .{field.svdName}(&self._{field.name | snake})
            {{ endfor }}\
            {{ for field in register.writeFields }}\
            .{field.svdName}(&self._{field.name | snake})
            {{ endfor }}.write();\
{{ endfor }}\
{{ endfor }}\
{{endif}}
        }
    }\
";

pub fn render_template_into_path<C>(template_id: &str, content: &C, path: &String)
where
    C: Serialize,
{
    let mut file = File::create(path).unwrap();
    render_template_into_file(template_id, content, &mut file);
}

/*fn formatter_camel(value: &Value, output: &mut String) -> Result<()> {
    match value {
        Value::String(s) => {
            output.push_str(&camelcase::to_camel_case(s));
        }
        _ => panic!("Unsupported value type"),
    }
    Ok(())
}

fn formatter_pascal(value: &Value, output: &mut String) -> Result<()> {
    match value {
        Value::String(s) => {
            output.push_str(&pascalcase::to_pascal_case(s));
        }
        _ => panic!("Unsupported value type"),
    }
    Ok(())
}

fn formatter_snake(value: &Value, output: &mut String) -> Result<()> {
    match value {
        Value::String(s) => {
            let value = &snakecase::to_snake_case(s);
            output.push_str(&value.clone());
        }
        _ => panic!("Unsupported value type"),
    }
    Ok(())
}

fn formatter_kebab(value: &Value, output: &mut String) -> Result<()> {
    match value {
        Value::String(s) => {
            output.push_str(&kebabcase::to_kebab_case(s));
        }
        _ => panic!("Unsupported value type"),
    }
    Ok(())
}*/

pub fn render_template_into_file<C>(template_id: &str, content: &C, file: &mut File)
where
    C: Serialize,
{
    let result: String = TEMPLATES
        .render(template_id, &Context::from_serialize(&content).unwrap())
        .unwrap();
    file.write(result.as_bytes()).unwrap();
}
