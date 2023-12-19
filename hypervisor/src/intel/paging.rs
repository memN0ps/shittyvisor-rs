//! Intel® 64 and IA-32 Architectures Software Developer's Manual: 4.2 HIERARCHICAL PAGING STRUCTURES: AN OVERVIEW
//! This section covers the standard paging mechanism as used in x86-64 architecture.
//! Standard paging controls how virtual memory addresses are translated to physical memory addresses.
//!
//! Credits to the work by Satoshi in their 'Hello-VT-rp' project for assistance and a clear implementation of this Paging Structure:
//! https://github.com/tandasat/Hello-VT-rp/blob/main/hypervisor/src/paging_structures.rs

use {
    crate::utils::addresses::PhysicalAddress,
    bitfield::bitfield,
    core::ptr::addr_of,
    x86::current::paging::{BASE_PAGE_SHIFT, LARGE_PAGE_SIZE},
};

/// Represents the entire Extended Page Table structure.
///
/// EPT is a set of nested page tables similar to the standard x86-64 paging mechanism.
/// It consists of 4 levels: PML4, PDPT, PD, and PT.
///
/// Reference: Intel® 64 and IA-32 Architectures Software Developer's Manual: 29.3.2 EPT Translation Mechanism
#[repr(C, align(4096))]
#[derive(Debug, Clone, Copy)]
pub struct PageTables {
    /// Page Map Level 4 (PML4) Table.
    pml4: Pml4,
    /// Page Directory Pointer Table (PDPT).
    pdpt: Pdpt,
    /// Array of Page Directory Table (PDT).
    pd: [Pd; 512],
    /// Page Table (PT).
    pt: Pt,
}

impl PageTables {
    /// Builds a basic identity map for the page tables.
    ///
    /// This setup ensures that each virtual address directly maps to the same physical address,
    /// a common setup for the initial stages of an operating system or hypervisor.
    pub fn build_identity(&mut self) {
        // Configure the first entry in the PML4 table.
        // Set it to present and writable, pointing to the base of the PDPT.
        self.pml4.0.entries[0].set_present(true);
        self.pml4.0.entries[0].set_writable(true);
        self.pml4.0.entries[0]
            .set_pfn(PhysicalAddress::pa_from_va(addr_of!(self.pdpt) as u64) >> BASE_PAGE_SHIFT);

        // Start mapping physical addresses from 0.
        let mut pa = 0;

        // Iterate over each PDPT entry.
        for (i, pdpte) in self.pdpt.0.entries.iter_mut().enumerate() {
            // Set each PDPT entry to present and writable,
            // pointing to the corresponding page directory (PD).
            pdpte.set_present(true);
            pdpte.set_writable(true);
            pdpte.set_pfn(
                PhysicalAddress::pa_from_va(addr_of!(self.pd[i]) as u64) >> BASE_PAGE_SHIFT,
            );

            // Configure each entry in the PD to map a large page (e.g., 2MB).
            for pde in &mut self.pd[i].0.entries {
                // Set each PD entry to present, writable, and as a large page.
                // Point it to the corresponding physical address.
                pde.set_present(true);
                pde.set_writable(true);
                pde.set_large(true);
                pde.set_pfn(pa >> BASE_PAGE_SHIFT);

                // Increment the physical address by the size of a large page.
                pa += LARGE_PAGE_SIZE as u64;
            }
        }
    }
}

/// Represents a PML4 Entry (PML4E) that references a Page-Directory-Pointer Table.
///
/// PML4 is the top level in the standard x86-64 paging hierarchy.
///
/// Reference: Intel® 64 and IA-32 Architectures Software Developer's Manual: 4.5 Paging
#[repr(C, align(4096))]
#[derive(Debug, Clone, Copy)]
struct Pml4(Table);

/// Represents a Page-Directory-Pointer-Table Entry (PDPTE) that references a Page Directory.
///
/// PDPTEs are part of the second level in the standard x86-64 paging hierarchy.
///
/// Reference: Intel® 64 and IA-32 Architectures Software Developer's Manual: 4.5 Paging
#[repr(C, align(4096))]
#[derive(Debug, Clone, Copy)]
struct Pdpt(Table);

/// Represents a Page-Directory Entry (PDE) that references a Page Table.
///
/// PDEs are part of the third level in the standard x86-64 paging hierarchy.
///
/// Reference: Intel® 64 and IA-32 Architectures Software Developer's Manual: 4.5 Paging
#[repr(C, align(4096))]
#[derive(Debug, Clone, Copy)]
struct Pd(Table);

/// Represents a Page-Table Entry (PTE) that maps a 4-KByte Page.
///
/// PTEs are the lowest level in the standard x86-64 paging hierarchy and are used to map individual
/// pages to physical addresses.
///
/// Reference: Intel® 64 and IA-32 Architectures Software Developer's Manual: 4.5 Paging
#[repr(C, align(4096))]
#[derive(Debug, Clone, Copy)]
struct Pt(Table);

/// General struct to represent a table in the standard paging structure.
///
/// This struct is used as a basis for PML4, PDPT, PD, and PT. It contains an array of entries
/// where each entry can represent different levels of the paging hierarchy.
#[repr(C, align(4096))]
#[derive(Debug, Clone, Copy)]
struct Table {
    entries: [Entry; 512],
}

bitfield! {
    /// Represents a Page Table Entry in standard paging.
    ///
    /// These entries are used to manage memory access and address mapping.
    ///
    /// # Fields
    ///
    /// * `present` - If set, the memory region is accessible.
    /// * `writable` - If set, the memory region can be written to.
    /// * `large` - If set, this entry maps a large page.
    /// * `pfn` - The Page Frame Number, indicating the physical address.
    ///
    /// Reference: Intel® 64 and IA-32 Architectures Software Developer's Manual: 4.5 Paging
    #[repr(C, align(4096))]
    #[derive(Clone, Copy, Default)]
    pub struct Entry(u64);
    impl Debug;

    present, set_present: 0;
    writable, set_writable: 1;
    large, set_large: 7;
    pfn, set_pfn: 51, 12;
}