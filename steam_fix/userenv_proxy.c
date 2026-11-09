/*
fixes steam's black window under winehq on a mac by adding --in-process-gpu to steamwebhelper's browser process.
every export forwards to wine's own userenv, copied next to this as userenv_orig.dll.
*/
#include <windows.h>
#include <winternl.h>
#include <wchar.h>

static const WCHAR flag[] = L" --in-process-gpu";

static int replace_pointer(HMODULE module, const void *from, void *to)
{
    BYTE *base = (BYTE *)module;
    IMAGE_DOS_HEADER *dos = (IMAGE_DOS_HEADER *)base;
    IMAGE_NT_HEADERS *nt = (IMAGE_NT_HEADERS *)(base + dos->e_lfanew);
    IMAGE_SECTION_HEADER *section = IMAGE_FIRST_SECTION(nt);
    int replaced = 0;
    for (WORD i = 0; i < nt->FileHeader.NumberOfSections; i++, section++) {
        if (!(section->Characteristics & IMAGE_SCN_MEM_WRITE))
            continue;
        void **slot = (void **)(base + section->VirtualAddress);
        void **end = (void **)(base + section->VirtualAddress + section->Misc.VirtualSize);
        for (; slot < end; slot++) {
            if (*slot == from) {
                *slot = to;
                replaced++;
            }
        }
    }
    return replaced;
}

static void add_in_process_gpu(void)
{
    RTL_USER_PROCESS_PARAMETERS *params = NtCurrentTeb()->ProcessEnvironmentBlock->ProcessParameters;
    UNICODE_STRING *line = &params->CommandLine;
    if (!line->Buffer || wcsstr(line->Buffer, L"--type=") || wcsstr(line->Buffer, L"--in-process-gpu"))
        return;
    size_t old_chars = line->Length / sizeof(WCHAR);
    size_t flag_chars = (sizeof(flag) / sizeof(WCHAR)) - 1;
    size_t bytes = (old_chars + flag_chars + 1) * sizeof(WCHAR);
    WCHAR *buffer = HeapAlloc(GetProcessHeap(), 0, bytes);
    if (!buffer)
        return;
    memcpy(buffer, line->Buffer, old_chars * sizeof(WCHAR));
    memcpy(buffer + old_chars, flag, flag_chars * sizeof(WCHAR));
    buffer[old_chars + flag_chars] = 0;
    WCHAR *old = GetCommandLineW();
    HMODULE kernelbase = GetModuleHandleW(L"kernelbase.dll");
    if (old == line->Buffer && kernelbase)
        replace_pointer(kernelbase, old, buffer);
    line->Buffer = buffer;
    line->Length = (USHORT)((old_chars + flag_chars) * sizeof(WCHAR));
    line->MaximumLength = (USHORT)bytes;
}

BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, LPVOID reserved)
{
    (void)instance;
    (void)reserved;
    if (reason == DLL_PROCESS_ATTACH)
        add_in_process_gpu();
    return TRUE;
}
