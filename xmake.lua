set_project("tyos")
set_version("0.0.1")

add_rules("mode.debug", "mode.release")

set_policy("check.auto_ignore_flags", false)

toolchain("m68k-toolchain")
set_kind("standalone")

set_sdkdir("/opt/homebrew")

set_toolset("cc", "m68k-elf-gcc")
set_toolset("ld", "m68k-elf-ld")
set_toolset("as", "m68k-elf-gcc")
set_toolset("ar", "m68k-elf-ar")
set_toolset("objcopy", "m68k-elf-objcopy")
set_toolset("strip", "m68k-elf-strip")
toolchain_end()

target("kernel.elf")
set_kind("binary")
set_toolchains("m68k-toolchain")

add_files("boot/*.s")
add_files("kernel/*.c")

add_cflags("-m68000", "-O2", "-ffreestanding", "-Wall")
add_asflags("-m68000")

add_ldflags("-Tlinker.ld", "-nostdlib")

after_build(function(target)
    import("core.project.task")
    cprint("${green}[*] Generating raw binary: kernel.bin")

    local objcopy = target:tool("objcopy")
    local elf_path = target:targetfile()
    local bin_path = elf_path:gsub("elf", "bin")

    os.execv(objcopy, { "-O", "binary", elf_path, bin_path })
    cprint("${bright}    -> Output: %s", bin_path)
end)
