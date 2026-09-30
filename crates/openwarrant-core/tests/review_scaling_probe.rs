// SPDX-License-Identifier: Apache-2.0
use openwarrant_core::document::*;
#[test]
fn scaling_probe(){
 let base=parse_document(include_bytes!("../../../conformance/sdk/document/minimal-rc3.md"),Dialect::Rc3,ParseLimits::default()).unwrap();
 for count in [2000,8000] {
 let fields=DocumentFields{metadata:base.metadata().as_table().unwrap().iter().map(|(k,v)|(k.clone(),if k=="kind" { MetadataValue::from("context") } else { v.clone() })).collect(),units:(0..count).map(|i|AuthoredUnit{id:format!("u-{i}"),kind:UnitKind::Binding,text:if i==0{base.unit(&base.units()[0].id).unwrap().into()}else{"## Section\nBody.\n".into()}}).collect()};
 let start=std::time::Instant::now();let bytes=author_document(&fields,AuthorOptions::default(),ParseLimits::default()).unwrap();let authored=start.elapsed();
 let doc=parse_document(&bytes,Dialect::Rc3,ParseLimits::default()).unwrap();
 let start=std::time::Instant::now();let edit=edit_document(&doc,&[],ParseLimits::default()).unwrap();let edited=start.elapsed();assert_eq!(edit,bytes);
 eprintln!("units={count}, bytes={}, author={authored:?}, noop-edit={edited:?}",bytes.len());
 }
}
