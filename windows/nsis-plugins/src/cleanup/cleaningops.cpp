#include "stdafx.h"
#include "cleaningops.h"
#include <libcommon/filesystem.h>
#include <libcommon/fileenumerator.h>
#include <libcommon/string.h>
#include <libcommon/memory.h>
#include <libcommon/security.h>
#include <libcommon/process.h>
#include <experimental/filesystem>
#include <utility>
#include <functional>
#include <processthreadsapi.h>

namespace
{

//
// Returns range in lhs that is also present in rhs.
// Equalence/equivalence determined by 'comp'.
//
// Returns pair<lhsBegin, lhsBegin> if there is no mirrored range.
//
template<typename ForwardIterator>
std::pair<ForwardIterator, ForwardIterator>
mirrored_range
(
	ForwardIterator lhsBegin, ForwardIterator lhsEnd,
	ForwardIterator rhsBegin, ForwardIterator rhsEnd,
	std::function<bool(const typename ForwardIterator::value_type &, const typename ForwardIterator::value_type &)> comp
)
{
	ForwardIterator begin = lhsBegin;

	while (lhsBegin != lhsEnd
		&& rhsBegin != rhsEnd)
	{
		if (false == comp(*lhsBegin, *rhsBegin))
		{
			break;
		}

		++lhsBegin;
		++rhsBegin;
	}

	return std::make_pair(begin, lhsBegin);
}

std::wstring ConstructLocalAppDataPath(const std::wstring &base, const std::wstring &user,
	const std::pair<std::vector<std::wstring>::iterator, std::vector<std::wstring>::iterator> &tokens)
{
	auto path = std::experimental::filesystem::path(base);

	path.append(user);

	std::for_each(tokens.first, tokens.second, [&](const std::wstring &token)
	{
		path.append(token);
	});

	return path;
}

std::wstring GetSystemUserLocalAppData()
{
	common::security::AdjustCurrentProcessTokenPrivilege(L"SeDebugPrivilege");

	common::memory::ScopeDestructor sd;

	sd += []
	{
		common::security::AdjustCurrentProcessTokenPrivilege(L"SeDebugPrivilege", false);
	};

	auto systemDir = common::fs::GetKnownFolderPath(FOLDERID_System, KF_FLAG_DEFAULT, NULL);
	auto lsassPath = std::experimental::filesystem::path(systemDir).append(L"lsass.exe");
	auto lsassPid = common::process::GetProcessIdFromName(lsassPath);

	auto processHandle = OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, lsassPid);

	if (nullptr == processHandle)
	{
		throw std::runtime_error("Failed to access the \"LSASS\" process");
	}

	HANDLE processToken;

	auto status = OpenProcessToken(processHandle, TOKEN_READ | TOKEN_IMPERSONATE | TOKEN_DUPLICATE, &processToken);

	CloseHandle(processHandle);

	if (FALSE == status)
	{
		throw std::runtime_error("Failed to acquire process token for the \"LSASS\" process");
	}

	sd += [&]()
	{
		CloseHandle(processToken);
	};

	return common::fs::GetKnownFolderPath(FOLDERID_LocalAppData, KF_FLAG_DEFAULT, processToken);
}

} // anonymous namespace

namespace cleaningops
{

void RemoveLogsCacheCurrentUser()
{
	const auto localAppData = common::fs::GetKnownFolderPath(FOLDERID_LocalAppData, KF_FLAG_DEFAULT, nullptr);
	const auto appdir = std::experimental::filesystem::path(localAppData).append(L"Mullvad VPN");

	std::experimental::filesystem::remove_all(appdir);
}

void RemoveLogsCacheOtherUsers()
{
	//
	// Determine relative path to "local app data" from home directory.
	//
	// Beware, the local app data path may be overriden from its default location
	// as a node somewhere beneath the home directory.
	//

	auto localAppData = common::fs::GetKnownFolderPath(FOLDERID_LocalAppData, KF_FLAG_DEFAULT, nullptr);
	auto homeDir = common::fs::GetKnownFolderPath(FOLDERID_Profile, KF_FLAG_DEFAULT, nullptr);

	//
	// Tokenize to get rid of slashes pointing in different directions.
	//
	auto localAppDataTokens = common::string::Tokenize(localAppData, L"\\/");
	auto homeDirTokens = common::string::Tokenize(homeDir, L"\\/");

	auto mirror = mirrored_range
	(
		localAppDataTokens.begin(), localAppDataTokens.end(),
		homeDirTokens.begin(), homeDirTokens.end(),
		[](const std::wstring &lhs, const std::wstring &rhs)
		{
			return 0 == _wcsicmp(lhs.c_str(), rhs.c_str());
		}
	);

	auto equalTokensCount = (size_t)std::distance(mirror.first, mirror.second);

	//
	// Abort if "local app data" is not beneath home dir.
	//
	if (equalTokensCount < homeDirTokens.size())
	{
		return;
	}

	auto relativeLocalAppData = std::make_pair(std::next(localAppDataTokens.begin(), equalTokensCount), localAppDataTokens.end());
	auto currentUser = *homeDirTokens.rbegin();

	//
	// Find all other users and construct the most plausible path for their
	// respective "local app data" dirs.
	//

	auto parentHomeDir = common::fs::GetKnownFolderPath(FOLDERID_UserProfiles, KF_FLAG_DEFAULT, nullptr);

	common::fs::FileEnumerator files(parentHomeDir);

	files.addFilter(std::make_unique<common::fs::FilterDirectories>());
	files.addFilter(std::make_unique<common::fs::FilterNotRelativeDirs>());

	auto notNamedSet = std::make_unique<common::fs::FilterNotNamedSet>();

	notNamedSet->addObject(std::wstring(currentUser));
	notNamedSet->addObject(L"All Users"); // Redirects to 'c:\programdata'.
	notNamedSet->addObject(L"Public"); // Shared documents, not an actual user or user template.

	files.addFilter(std::move(notNamedSet));

	WIN32_FIND_DATAW file;

	while (files.next(file))
	{
		const auto userLocalAppData = ConstructLocalAppDataPath(files.getDirectory(), file.cFileName, relativeLocalAppData);
		const auto target = std::experimental::filesystem::path(userLocalAppData).append(L"Mullvad VPN");

		std::error_code dummy;
		std::experimental::filesystem::remove_all(target, dummy);
	}
}

void RemoveLogsServiceUser()
{
	const auto programData = common::fs::GetKnownFolderPath(FOLDERID_ProgramData, KF_FLAG_DEFAULT, nullptr);
	const auto appdir = std::experimental::filesystem::path(programData).append(L"Mullvad VPN");

	std::experimental::filesystem::remove_all(appdir);
}

void RemoveCacheServiceUser()
{
	const auto localAppData = GetSystemUserLocalAppData();
	const auto mullvadAppData = std::experimental::filesystem::path(localAppData).append(L"Mullvad VPN");

	common::fs::ScopedNativeFileSystem nativeFileSystem;

	common::security::AddAdminToObjectDacl(mullvadAppData, SE_FILE_OBJECT);

	{
		common::fs::FileEnumerator files(mullvadAppData);

		auto notNamedSet = std::make_unique<common::fs::FilterNotNamedSet>();

		notNamedSet->addObject(L"account-history.json");
		notNamedSet->addObject(L"settings.json");

		files.addFilter(std::move(notNamedSet));
		files.addFilter(std::make_unique<common::fs::FilterFiles>());

		WIN32_FIND_DATAW file;

		while (files.next(file))
		{
			const auto target = std::experimental::filesystem::path(files.getDirectory()).append(file.cFileName);

			std::error_code dummy;
			std::experimental::filesystem::remove(target, dummy);
		}
	}

	//
	// This fails unless the directory is empty.
	// Which is what we want, since removing cache and settings files are separate operations.
	//
	RemoveDirectoryW(std::wstring(L"\\\\?\\").append(mullvadAppData).c_str());
}

void RemoveSettingsServiceUser()
{
	const auto localAppData = GetSystemUserLocalAppData();
	const auto mullvadAppData = std::experimental::filesystem::path(localAppData).append(L"Mullvad VPN");

	common::fs::ScopedNativeFileSystem nativeFileSystem;

	common::security::AddAdminToObjectDacl(mullvadAppData, SE_FILE_OBJECT);

	{
		common::fs::FileEnumerator files(mullvadAppData);

		auto filter = std::make_unique<common::fs::FilterNamedSet>();

		filter->addObject(L"account-history.json");
		filter->addObject(L"settings.json");

		files.addFilter(std::move(filter));
		files.addFilter(std::make_unique<common::fs::FilterFiles>());

		WIN32_FIND_DATAW file;

		while (files.next(file))
		{
			const auto target = std::experimental::filesystem::path(files.getDirectory()).append(file.cFileName);

			std::error_code dummy;
			std::experimental::filesystem::remove(target, dummy);
		}
	}

	//
	// This fails unless the directory is empty.
	// Which is what we want, since removing cache and settings files are separate operations.
	//
	RemoveDirectoryW(std::wstring(L"\\\\?\\").append(mullvadAppData).c_str());
}

void RemoveRelayCacheServiceUser()
{
	const auto localAppData = GetSystemUserLocalAppData();
	const auto mullvadAppData = std::experimental::filesystem::path(localAppData).append(L"Mullvad VPN");

	common::fs::ScopedNativeFileSystem nativeFileSystem;

	common::security::AddAdminToObjectDacl(mullvadAppData, SE_FILE_OBJECT);

	const auto cacheFile = std::experimental::filesystem::path(mullvadAppData).append(L"relays.json");

	std::experimental::filesystem::remove(cacheFile);
}

}
