OUTPUT_FORMAT("elf64-powerpc")
OUTPUT_ARCH(powerpc:common64)
ENTRY(_start)
PHDRS {
    text PT_LOAD FLAGS(0x00400005);
    data PT_LOAD FLAGS(0x00600006);
}
SECTIONS {
    . = 0x10000;
    .text : { *(.text .text.*) } :text
    .rodata : { *(.entrydesc .rodata .rodata.*) } :text
    . = ALIGN(0x10000);
    .opd : { *(.opd) } :data
    .got : { *(.got .toc) } :data
    __toc = ADDR(.got) + 0x8000;
    .data : { *(.data*) } :data
    .bss : { *(.bss*) } :data
    /DISCARD/ : { *(.eh_frame .note.GNU-stack) }
}
