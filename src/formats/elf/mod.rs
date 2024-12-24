#![allow(dead_code)]

use std::{
    collections::{btree_map::Entry, BTreeMap}, fs::File, io::{BufWriter, Write}, path::Path
};

use crate::{
    compiler::CompilerContext, formats::elf::sections::SectionHeader,
    st_info, st_visibility, utils::IBytes,
};

pub mod flags;
pub mod header;
pub mod program;
pub mod sections;

use self::{
    flags::{STB_GLOBAL, STB_LOCAL, STT_FILE, STT_NOTYPE, STT_SECTION, STV_DEFAULT},
    header::ElfHeader,
    sections::{NOBITSSec, PROGBITSSec, RELASec, STRTABSec, SYMTABSec, Section, SymItem},
};

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolType {
    Global,
    Ffi,
    DataSec,
    BssSec,
    TextSec,
    Other,
}

pub fn generate_bin(out_path: &Path, cc: &mut CompilerContext) {
    let file_content = cc.codegen.text_section_bytes();
    let stream = File::create(out_path.with_extension("bin")).unwrap();
    let mut file = BufWriter::new(stream);
    file.write_all(&file_content).unwrap();
    file.flush().unwrap();
}

pub fn generate_elf(out_path: &Path, cc: &mut CompilerContext) {
    let mut dyn_sections: Vec<Box<dyn Section>> = vec![
        Box::new(PROGBITSSec::new(".text", 0x6, 16, cc.codegen.text_section_bytes()))
    ];
    if !cc.codegen.data_buf.is_empty() {
        dyn_sections.push(
            Box::new(PROGBITSSec::new(".data", 0x3, 4, PROGBITSSec::dmap_to_data(&cc.codegen.data_buf))));
    }
    if !cc.codegen.bss_buf.is_empty() {
        dyn_sections.push(
            Box::new(NOBITSSec::new(".bss", cc.codegen.bss_buf.iter().map(|x| x.size).sum())));
    }

    let mut strtab = STRTABSec::new(".strtab");
    strtab.insert(&cc.program_file);
    let symtab = set_symbols(&mut strtab, &dyn_sections, cc);
    let mut rela_map = RELASec{ name: ".rela.text".into(), data: cc.codegen.rela_map.clone()};
    let mut shstrtab = STRTABSec::new(".shstrtab");

    let mut sections = BTreeMap::<String, Box<dyn Section>>::new();
    for sec in dyn_sections.iter() {
        shstrtab.insert(&sec.name());
        sections.insert(sec.name(), sec.clone_box());
    }
    shstrtab.insert(".shstrtab");
    shstrtab.insert(".symtab");
    shstrtab.insert(".rela.text");
    shstrtab.insert(".strtab");
    if !cc.codegen.rela_map.is_empty() {
        for item in cc.codegen.rela_map.iter_mut() {
            if item.sym_type == SymbolType::Ffi {
                let indx = strtab.index(&item.sym_name).unwrap();
                item.r_section = symtab.find(indx) as u32;
            } else {
                item.r_section = sections.values().position(|t| t.name() == item.sym_name).unwrap() as u32 + 1;
            }
           rela_map.push(item.to_owned());
        }
    }
    sections.insert(".shstrtab".into(), Box::new(shstrtab));
    sections.insert(".symtab".into(), Box::new(symtab));
    if !cc.codegen.rela_map.is_empty() {
        sections.insert(".rela.text".into(), Box::new(rela_map));
    }
    sections.insert(".strtab".into(), Box::new(strtab));

    let elf_sections = ElfSections::new(64 + (64 * (sections.len() + 1)) as u64, sections);
    let section_headers = elf_sections.section_headers();
    let elf_header = ElfHeader::new(section_headers.len() as u16,
        elf_sections.get_sec_index(".shstrtab") as u16);

    let stream = File::create(out_path.with_extension("o")).unwrap();
    let mut file = BufWriter::new(stream);
    file.write_all(&elf_header.to_bytes()).unwrap();
    file.write_all(&elf_sections.get_section_header_bytes()).unwrap();
    file.write_all(&elf_sections.bytes()).unwrap();
    file.flush().unwrap();
}

pub struct ElfSections {
    sections: BTreeMap<String, Box<dyn Section>>,
    offset: u64,
}

impl ElfSections {
    pub fn new(offset: u64, sections: BTreeMap<String, Box<dyn Section>>) -> Self {
        Self {
            sections,
            offset,
        }
    }

    pub fn section_sizes(&self) -> usize {
        let mut size = 0;
        for sec in self.sections.values() {
            size += sec.size();
        }
        size
    }

    pub fn section_name_index(&self, name: &str) -> u32 {
        let strtab = self.sections.get(".shstrtab").expect("shstrtab section must be present")
            .as_any().downcast_ref::<STRTABSec>().expect("wrong .shstrtab type");
        strtab.index(name).expect("Section with this name dosent exist")
    }

    pub fn add_section<T>(&mut self, section: &T)
    where
        T: Section + Clone + 'static,
    {
        self.sections.insert(section.name(), Box::new((*section).clone()));
        match self.sections.entry(".shstrtab".into()) {
            Entry::Occupied(mut oe) => {
                let w = oe.get_mut();
                w.insert(section.name().as_bytes());
            },
            Entry::Vacant(_) => {
                panic!("shstrtab section must be present");
            }
        }
    }

    fn sections_count(&self) -> usize {
        self.sections.len()
    }

    pub fn bytes(&self) -> IBytes {
        let mut bytes = Vec::new();
        for sec in self.sections.values().rev() {
            bytes.extend(sec.to_bytes());
        }
        bytes
    }

    pub fn get_header(&self) -> ElfHeader {
        assert!(
            self.get_sec_index(".shstrtab") != 0,
            "Header is not ready yet!"
        );
        ElfHeader::new(
            self.sections_count() as u16,
            self.get_sec_index(".shstrtab") as u16,
        )
    }

    pub fn get_sec_index(&self, tag: &str) -> u32 {
       match self.sections.values().rev().position(|x| x.name() == tag) {
           Some(pos) => {
               pos as u32 + 1
           }
           None => {
               0
           }
       }
    }

    pub fn section_headers(&self) -> Vec<SectionHeader> {
        let mut secs = vec![SectionHeader::default()];
        let mut offset = self.offset;
        for section in self.sections.values().rev() {
            let (link_tag, info_tag) = section.link_and_info();
            let link = self.get_sec_index(link_tag.unwrap_or(""));
            let info = self.get_sec_index(info_tag.unwrap_or(""));
            secs.push(section.header(
                self.section_name_index(&section.name()),
                offset,
                link,
                info,
            ));
            offset += section.padded_size() as u64;
        }
        secs
    }

    pub fn get_section_header_bytes(&self) -> IBytes {
        let mut bytes = Vec::<u8>::new();
        for sec in self.section_headers() {
            bytes.extend(sec.to_bytes());
        }
        bytes
    }

    fn section_header_bytes(&self, bytes: &mut IBytes) {
        for sec in self.section_headers().iter() {
            bytes.extend(sec.to_bytes());
        }
    }

}

pub fn set_symbols(
        strtab: &mut STRTABSec,
        dyn_sections: &[Box<dyn Section>],
        cc: &mut CompilerContext) -> SYMTABSec
{
    let mut symtab = SYMTABSec::new(".symtab");
    symtab.insert(SymItem {
        st_name: strtab.index(&cc.program_file).unwrap(),
        st_info: st_info!(STB_LOCAL, STT_FILE),
        st_other: st_visibility!(STV_DEFAULT),
        st_shndx: 0xfff1,
        st_size: 0,
        st_value: 0,
    });

    for (indx, _) in dyn_sections.iter().enumerate() {
        symtab.insert(SymItem {
            st_name: 0,
            st_info: st_info!(STB_LOCAL, STT_SECTION),
            st_other: st_visibility!(STV_DEFAULT),
            st_shndx: indx as u16 + 1,
            st_size: 0,
            st_value: 0,
        });
    }
    for (label, sym) in cc.codegen.symbols_map.iter() {
        // push symbol name to list
        strtab.insert(label);

        if label == "_start" || sym.1 == SymbolType::Ffi {
            continue;
        }
        // push symbol info to sym_list
        let info = match sym.1 == SymbolType::Ffi {
            true => st_info!(STB_GLOBAL, STT_NOTYPE),
            false => st_info!(STB_LOCAL, STT_NOTYPE),
        };
        let shndx_tag = match sym.1 {
            SymbolType::TextSec => ".text",
            SymbolType::DataSec => ".data",
            SymbolType::BssSec => ".bss",
            _ => "",
        };
        let shndx = dyn_sections.iter().position(|s| s.name() == shndx_tag).expect("shndx with that section dose not exist");

        symtab.insert(SymItem {
            st_name: strtab.index(label).unwrap(),
            st_info: info,
            st_other: st_visibility!(STV_DEFAULT),
            st_shndx: shndx as u16,
            st_size: 0,
            st_value: sym.0 as u64,
        });
    }
    // Global items
    symtab.set_global_start();
    for item in cc.codegen.ffi_map.values() {
        symtab.insert(SymItem {
            st_name: strtab.index(item).unwrap(),
            st_info: st_info!(STB_GLOBAL, STT_NOTYPE),
            st_other: st_visibility!(STV_DEFAULT),
            st_shndx: 0,
            st_size: 0,
            st_value: 0,
        });
    }
    symtab.insert(SymItem {
        st_name: strtab.index("_start").unwrap(),
        st_info: st_info!(STB_GLOBAL, STT_NOTYPE),
        st_other: st_visibility!(STV_DEFAULT),
        st_shndx: dyn_sections.iter().position(|s| s.name() == ".text").unwrap() as u16,
        st_size: 0,
        st_value: 0,
    });
    symtab
}
