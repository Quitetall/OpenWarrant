// SPDX-License-Identifier: Apache-2.0
use openwarrant_core::document::*;
#[test]
fn rc2_title_edit_changes_metadata_preserves_units() {
 let original=include_bytes!("../../../conformance/sdk/document/minimal-rc2.md");
 let doc=parse_document(original,Dialect::Rc2,ParseLimits::default()).unwrap();
 let bytes=edit_document(&doc,&[DocumentEdit::Metadata{key:"title".into(),value:Some("Changed legacy title".into())}],ParseLimits::default()).unwrap();
 let changed=parse_document(&bytes,Dialect::Rc2,ParseLimits::default()).unwrap();
 assert_eq!(changed.metadata()["title"].as_str(),Some("Changed legacy title"));
 assert_ne!(changed.metadata()["title"],doc.metadata()["title"]);
 assert_eq!(validate_document(&changed,&ValidationOptions::default()).validity,Validity::Valid);
 for unit in doc.units(){ assert_eq!(changed.unit(&unit.id),doc.unit(&unit.id)); }
 assert_eq!(doc.original(),original);
 println!("RC2 title changed to {:?}; all {} units byte-identical; validity {:?}",changed.metadata()["title"].as_str(),doc.units().len(),validate_document(&changed,&ValidationOptions::default()).validity);
}
