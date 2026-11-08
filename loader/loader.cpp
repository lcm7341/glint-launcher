// xinput9_1_0.dll proxy that loads glint-mods, for gd before 2.2

#include <windows.h>
#include <algorithm>
#include <string>
#include <vector>

#define XINPUT_NOT_CONNECTED 1167

// glint looks for this string
extern "C" const char glint_loader[] = "GlintLoader 1";

typedef DWORD(WINAPI* GetStateFunction)(DWORD, void*);
typedef DWORD(WINAPI* SetStateFunction)(DWORD, void*);
typedef DWORD(WINAPI* GetCapabilitiesFunction)(DWORD, DWORD, void*);
typedef DWORD(WINAPI* GetGuidsFunction)(DWORD, GUID*, GUID*);

static HMODULE g_self = nullptr;
static GetStateFunction g_get_state = nullptr;
static SetStateFunction g_set_state = nullptr;
static GetCapabilitiesFunction g_get_capabilities = nullptr;
static GetGuidsFunction g_get_guids = nullptr;

static std::wstring module_folder(HMODULE module) {
	std::wstring path(MAX_PATH, L'\0');
	while (true) {
		const DWORD length = GetModuleFileNameW(module, path.data(), (DWORD)path.size());
		if (length == 0) return L"";
		if (length < path.size()) {
			path.resize(length);
			break;
		}
		path.resize(path.size() * 2);
	}
	const size_t slash = path.find_last_of(L"\\/");
	return slash == std::wstring::npos ? L"" : path.substr(0, slash);
}

static void load_system_xinput() {
	wchar_t system_folder[MAX_PATH];
	const UINT length = GetSystemDirectoryW(system_folder, MAX_PATH);
	if (length == 0 || length >= MAX_PATH) return;
	const std::wstring path = std::wstring(system_folder) + L"\\XInput9_1_0.dll";
	const HMODULE xinput = LoadLibraryW(path.c_str());
	if (!xinput) return;
	g_get_state = (GetStateFunction)GetProcAddress(xinput, "XInputGetState");
	g_set_state = (SetStateFunction)GetProcAddress(xinput, "XInputSetState");
	g_get_capabilities = (GetCapabilitiesFunction)GetProcAddress(xinput, "XInputGetCapabilities");
	g_get_guids = (GetGuidsFunction)GetProcAddress(xinput, "XInputGetDSoundAudioDeviceGuids");
}

static bool file_exists(const std::wstring& path) {
	return GetFileAttributesW(path.c_str()) != INVALID_FILE_ATTRIBUTES;
}

static void load_chained_loader(const std::wstring& game_folder) {
	const std::wstring path = game_folder + L"\\XInput9_1_0.dll.glint-backup";
	if (file_exists(path)) LoadLibraryW(path.c_str());
}

static bool ends_with_dll(const std::wstring& name) {
	return name.size() > 4 && _wcsicmp(name.c_str() + name.size() - 4, L".dll") == 0;
}

static std::vector<std::wstring> mod_names(const std::wstring& mods_folder) {
	std::vector<std::wstring> names;
	WIN32_FIND_DATAW found;
	const HANDLE search = FindFirstFileW((mods_folder + L"*.dll").c_str(), &found);
	if (search == INVALID_HANDLE_VALUE) return names;
	do {
		const std::wstring name = found.cFileName;
		if ((found.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) || !ends_with_dll(name)) continue;
		names.push_back(name);
	} while (FindNextFileW(search, &found));
	FindClose(search);
	std::sort(names.begin(), names.end(), [](const std::wstring& a, const std::wstring& b) {
		return _wcsicmp(a.c_str(), b.c_str()) < 0;
	});
	return names;
}

class Log {
public:
	explicit Log(const std::wstring& path) {
		file = CreateFileW(path.c_str(), GENERIC_WRITE, FILE_SHARE_READ, nullptr, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
	}
	~Log() {
		if (file != INVALID_HANDLE_VALUE) CloseHandle(file);
	}
	Log(const Log&) = delete;
	Log& operator=(const Log&) = delete;

	void line(const std::wstring& text) {
		if (file == INVALID_HANDLE_VALUE) return;
		const std::wstring with_end = text + L"\r\n";
		const int size = WideCharToMultiByte(CP_UTF8, 0, with_end.c_str(), (int)with_end.size(), nullptr, 0, nullptr, nullptr);
		if (size <= 0) return;
		std::string bytes(size, '\0');
		WideCharToMultiByte(CP_UTF8, 0, with_end.c_str(), (int)with_end.size(), bytes.data(), size, nullptr, nullptr);
		DWORD written = 0;
		WriteFile(file, bytes.data(), (DWORD)bytes.size(), &written, nullptr);
	}

private:
	HANDLE file = INVALID_HANDLE_VALUE;
};

static DWORD WINAPI load_mods(void*) {
	const std::wstring game_folder = module_folder(g_self);
	if (game_folder.empty()) return 0;
	const std::wstring mods_folder = game_folder + L"\\glint-mods\\";
	if (!file_exists(mods_folder)) return 0;
	Log log(mods_folder + L"glint-loader.log");
	const std::vector<std::wstring> names = mod_names(mods_folder);
	if (names.empty()) log.line(L"No mods to load");
	for (const std::wstring& name : names) {
		if (GetModuleHandleW(name.c_str())) {
			log.line(L"Skipped " + name + L": already loaded");
			continue;
		}
		if (LoadLibraryW((mods_folder + name).c_str())) log.line(L"Loaded " + name);
		else log.line(L"Couldn't load " + name + L": error " + std::to_wstring(GetLastError()));
	}
	return 0;
}

BOOL APIENTRY DllMain(HMODULE module, DWORD reason, LPVOID) {
	if (reason != DLL_PROCESS_ATTACH) return TRUE;
	g_self = module;
	DisableThreadLibraryCalls(module);
	load_system_xinput();
	load_chained_loader(module_folder(module));
	// runs once the loader lock is free
	const HANDLE thread = CreateThread(nullptr, 0, load_mods, nullptr, 0, nullptr);
	if (thread) CloseHandle(thread);
	return TRUE;
}

extern "C" DWORD WINAPI XInputGetState(DWORD user, void* state) {
	return g_get_state ? g_get_state(user, state) : XINPUT_NOT_CONNECTED;
}

extern "C" DWORD WINAPI XInputSetState(DWORD user, void* vibration) {
	return g_set_state ? g_set_state(user, vibration) : XINPUT_NOT_CONNECTED;
}

extern "C" DWORD WINAPI XInputGetCapabilities(DWORD user, DWORD flags, void* capabilities) {
	return g_get_capabilities ? g_get_capabilities(user, flags, capabilities) : XINPUT_NOT_CONNECTED;
}

extern "C" DWORD WINAPI XInputGetDSoundAudioDeviceGuids(DWORD user, GUID* render, GUID* capture) {
	return g_get_guids ? g_get_guids(user, render, capture) : XINPUT_NOT_CONNECTED;
}
