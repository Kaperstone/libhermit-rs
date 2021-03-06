// Copyright (c) 2017 Colin Finck, RWTH Aachen University
//
// MIT License
//
// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files (the
// "Software"), to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to
// the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
// LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
// WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use arch::x86_64::mm::paging::{BasePageSize, PageSize};
use collections::Node;
use hermit_multiboot::Multiboot;
use mm;
use mm::freelist::{FreeList, FreeListEntry};
use mm::POOL;


extern "C" {
	static limit: usize;
	static mb_info: usize;
}

static mut PHYSICAL_FREE_LIST: FreeList = FreeList::new();


fn detect_from_multiboot_info() -> Result<(), ()> {
	if unsafe { mb_info } == 0 {
		return Err(());
	}

	let mb = unsafe { Multiboot::new(mb_info) };
	let all_regions = mb.memory_map().expect("Could not find a memory map in the Multiboot information");
	let ram_regions = all_regions.filter(|m|
		m.is_available() &&
		m.base_address() + m.length() > mm::kernel_end_address()
	);
	let mut found_ram = false;

	for m in ram_regions {
		found_ram = true;

		let start_address = if m.base_address() <= mm::kernel_start_address() {
			mm::kernel_end_address()
		} else {
			m.base_address()
		};

		let entry = Node::new(
			FreeListEntry {
				start: start_address,
				end: m.base_address() + m.length()
			}
		);
		unsafe { PHYSICAL_FREE_LIST.list.push(entry); }
	}

	assert!(found_ram, "Could not find any available RAM in the Multiboot Memory Map");
	Ok(())
}

fn detect_from_limits() -> Result<(), ()> {
	if unsafe { limit } == 0 {
		return Err(());
	}

	let entry = Node::new(
		FreeListEntry {
			start: mm::kernel_end_address(),
			end: unsafe { limit }
		}
	);
	unsafe { PHYSICAL_FREE_LIST.list.push(entry); }

	Ok(())
}

pub fn init() {
	detect_from_multiboot_info()
		.or_else(|_e| detect_from_limits())
		.unwrap();
}

pub fn allocate(size: usize) -> usize {
	assert!(size > 0);
	assert!(size % BasePageSize::SIZE == 0, "Size {:#X} is not a multiple of {:#X}", size, BasePageSize::SIZE);

	let result = unsafe { PHYSICAL_FREE_LIST.allocate(size) };
	assert!(result.is_ok(), "Could not allocate {:#X} bytes of physical memory", size);
	result.unwrap()
}

pub fn allocate_aligned(size: usize, alignment: usize) -> usize {
	assert!(size > 0);
	assert!(alignment > 0);
	assert!(size % alignment == 0, "Size {:#X} is not a multiple of the given alignment {:#X}", size, alignment);
	assert!(alignment % BasePageSize::SIZE == 0, "Alignment {:#X} is not a multiple of {:#X}", alignment, BasePageSize::SIZE);

	let result = unsafe {
		POOL.maintain();
		PHYSICAL_FREE_LIST.allocate_aligned(size, alignment)
	};
	assert!(result.is_ok(), "Could not allocate {:#X} bytes of physical memory aligned to {} bytes", size, alignment);
	result.unwrap()
}

/// This function must only be called from mm::deallocate!
/// Otherwise, it may fail due to an empty node pool (POOL.maintain() is called in virtualmem::deallocate)
pub fn deallocate(physical_address: usize, size: usize) {
	assert!(physical_address >= mm::kernel_end_address(), "Physical address {:#X} is not >= KERNEL_END_ADDRESS", physical_address);
	assert!(size > 0);
	assert!(size % BasePageSize::SIZE == 0, "Size {:#X} is not a multiple of {:#X}", size, BasePageSize::SIZE);

	unsafe { PHYSICAL_FREE_LIST.deallocate(physical_address, size); }
}

pub fn print_information() {
	unsafe { PHYSICAL_FREE_LIST.print_information(" PHYSICAL MEMORY FREE LIST "); }
}
