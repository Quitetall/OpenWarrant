// SPDX-License-Identifier: Apache-2.0
use openwarrant_core::document::*;
use std::alloc::{GlobalAlloc,Layout,System};
use std::sync::atomic::{AtomicBool,AtomicUsize,Ordering};
struct Track;
static ON: AtomicBool=AtomicBool::new(false);
static ALLOC: AtomicUsize=AtomicUsize::new(0);
unsafe impl GlobalAlloc for Track {
 unsafe fn alloc(&self,l:Layout)->*mut u8 { if ON.load(Ordering::Relaxed){ALLOC.fetch_add(l.size(),Ordering::Relaxed);} unsafe{System.alloc(l)} }
 unsafe fn dealloc(&self,p:*mut u8,l:Layout){unsafe{System.dealloc(p,l)}}
 unsafe fn realloc(&self,p:*mut u8,l:Layout,n:usize)->*mut u8{if ON.load(Ordering::Relaxed){ALLOC.fetch_add(n,Ordering::Relaxed);}unsafe{System.realloc(p,l,n)}}
}
#[global_allocator] static A:Track=Track;
#[test]
fn resource_probe(){
 let limits=ParseLimits{source_bytes:1024,metadata_bytes:512,..ParseLimits::default()};
 let raw=include_bytes!("../../../conformance/sdk/document/minimal-rc3.md");
 let doc=parse_document(raw,Dialect::Rc3,limits).unwrap();
 for count in [10,100,1000] {
 let edits:Vec<_>=(0..count).map(|i|DocumentEdit::Metadata{key:format!("field{i}"),value:Some(MetadataValue::from("x".repeat(400)))}).collect();
 ALLOC.store(0,Ordering::Relaxed);ON.store(true,Ordering::Relaxed);
 let result=edit_document(&doc,&edits,limits);
 ON.store(false,Ordering::Relaxed);
 eprintln!("count={count}, allocated={}, result={:?}",ALLOC.load(Ordering::Relaxed),result.unwrap_err().code);
 }
}
