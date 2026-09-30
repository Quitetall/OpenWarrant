// SPDX-License-Identifier: Apache-2.0
use openwarrant_core::document::*;
#[test]
fn independent_author_edge_cases() {
    let original = include_bytes!("../../../conformance/sdk/document/minimal-rc3.md");
    let doc = parse_document(original, Dialect::Rc3, ParseLimits::default()).unwrap();
    let bytes=edit_document(&doc, &[DocumentEdit::Metadata{key:"id".into(),value:Some("example:changed".into())}],ParseLimits::default()).unwrap();
    let changed=parse_document(&bytes,Dialect::Rc3,ParseLimits::default()).unwrap();
    assert_eq!(changed.metadata()["id"].as_str(),Some("example:changed"));
    assert_eq!(changed.metadata()["revision"],doc.metadata()["revision"]);
    for u in doc.units() { assert_eq!(changed.unit(&u.id),doc.unit(&u.id)); }
    for key in ["id","title","schema","revision","state","kind"] {
        assert!(edit_document(&doc,&[DocumentEdit::Metadata{key:key.into(),value:None}],ParseLimits::default()).is_err());
        assert_eq!(doc.original(),original);
    }
    let mut fields=DocumentFields{metadata:doc.metadata().as_table().unwrap().iter().map(|(k,v)|(k.clone(),v.clone())).collect(),units:doc.units().iter().map(|u|AuthoredUnit{id:u.id.clone(),kind:u.kind,text:doc.unit(&u.id).unwrap().into()}).collect()};
    fields.units.push(fields.units[0].clone());
    assert!(author_document(&fields,AuthorOptions::default(),ParseLimits::default()).is_err());
    assert!(edit_document(&doc,&[DocumentEdit::Metadata{key:"title".into(),value:Some("é".repeat(257).into())}],ParseLimits::default()).is_err());
}
