use std::fmt::Display;
use dicom::object::mem::InMemDicomObject;
use dicom::core::value::{Value, PrimitiveValue, C};
use dicom::dictionary_std::StandardDataDictionary;
use dicom::core::dictionary::DataDictionary;

use crate::utils::Dicom;

pub fn get_dicom_table(dicom: &Dicom) -> Vec<[String; 3]> {

    let root = dicom.clone().into_inner();

    fn get_formatted_list(depth: usize, root: &MemDicom) -> Vec<[String; 3]> {

        let dict = StandardDataDictionary;
        let mut table = Vec::<[String; 3]>::new();
    
        let pad_depth = |s: String| format!("{}{}", " ".repeat(4*depth), s);
    
        for element in root {
    
            let tag_key = element.header().tag;
    
            let tag_name_str = dict
                .by_tag(tag_key.clone())
                .map(|entry| entry.alias)
                .unwrap_or("Unknown")
                .to_owned();
    
            let tag_key_str = pad_depth(format!("{}", tag_key));
            let val_str = format_value(element.value());
    
            table.push([tag_key_str, tag_name_str, val_str]);

            let separator = [
                " -".to_owned(),
                " -".to_owned(),
                " -".to_owned()
            ];
    
            if let Value::Sequence { items, .. } = element.value() {
                for item in items {
                    table.push(separator.clone());
                    let mut sub_table = get_formatted_list(depth + 1, item);
                    table.append(&mut sub_table);
                }
                table.push(separator.clone());
            }
        }
    
        table
    }

    get_formatted_list(0, &root)
}

type MemDicom = InMemDicomObject<StandardDataDictionary>;

fn format_value<P>(value: &Value<MemDicom, P>) -> String {

    match value {

        Value::Primitive(prim_val) => format_primitive(prim_val),
        Value::Sequence { .. } => "<sequence>".to_owned(),
        Value::PixelSequence { .. } => "<pixel sequence>".to_owned()
    }
}

fn format_primitive(prim_val: &PrimitiveValue) -> String {

    const MAX_STRING_DISPLAY_LEN: usize = 60;

    let s = match prim_val {

        PrimitiveValue::Empty => "<empty>".to_owned(),
        PrimitiveValue::Strs(arr) => format_array(arr),
        PrimitiveValue::Str(s) => s.clone(),
        PrimitiveValue::Tags(arr) => format_array(arr),
        PrimitiveValue::U8(arr) => format_array(arr),
        PrimitiveValue::I16(arr) => format_array(arr),
        PrimitiveValue::U16(arr) => format_array(arr),
        PrimitiveValue::I32(arr) => format_array(arr),
        PrimitiveValue::U32(arr) => format_array(arr),
        PrimitiveValue::I64(arr) => format_array(arr),
        PrimitiveValue::U64(arr) => format_array(arr),
        PrimitiveValue::F32(arr) => format_array(arr),
        PrimitiveValue::F64(arr) => format_array(arr),
        PrimitiveValue::Date(arr) => format_array(arr),
        PrimitiveValue::DateTime(arr) => format_array(arr),
        PrimitiveValue::Time(arr) => format_array(arr)
    };

    match s.len() > MAX_STRING_DISPLAY_LEN { 
        false => s,
        true => format!("{} <...>", &s[..MAX_STRING_DISPLAY_LEN])
    }
}

fn format_array<T: Display>(arr: &C<T>) -> String {

    const MAX_ARRAY_DISPLAY_LEN: usize = 8;

    match arr.len() {
        0 => "[]".to_owned(),
        1 => format!("{}", arr[0]),
        l => {

            let repr_list: Vec<String> = arr
                .iter()
                .take(MAX_ARRAY_DISPLAY_LEN)
                .map(|v| format!("{}", v))
                .collect();

            let s = repr_list.join(",");

            match l > MAX_ARRAY_DISPLAY_LEN { 
                false => s,
                true => format!("{},... <{} elements>", s, l)
            }
        }
    }.trim_end_matches(char::from(0)).to_owned()
} 
