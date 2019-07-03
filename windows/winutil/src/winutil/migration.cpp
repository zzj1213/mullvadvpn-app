#include "stdafx.h"
#include "migration.h"
#include <libcommon/filesystem.h>
#include <experimental/filesystem>
#include <stdexcept>

namespace fs = std::experimental::filesystem;

namespace migration {

//
// This is being called in a x64 SYSTEM user context
//
MigrationStatus MigrateAfterWindowsUpdate()
{
	const auto localAppData = common::fs::GetKnownFolderPath(FOLDERID_LocalAppData, KF_FLAG_DEFAULT, nullptr);
	const auto mullvadAppData = fs::path(localAppData).append(L"Mullvad VPN");

	//
	// The main settings file is 'settings.json'
	// If this file is present inside 'mullvadAppData' we should abort the migration
	//

	const auto settingsFile = fs::path(mullvadAppData).append(L"settings.json");

	if (fs::exists(settingsFile))
	{
		return MigrationStatus::Aborted;
	}

	//
	// Validate backup location path and ownership
	//

	const auto backupRoot = mullvadAppData.root_path().append(L"windows.old");
	const auto backupMullvadAppData = fs::path(backupRoot).append(mullvadAppData.relative_path());

	if (false == fs::exists(backupMullvadAppData))
	{
		return MigrationStatus::NothingToMigrate;
	}

	DWORD bufferSize = 128;
	std::vector<uint8_t> buffer(bufferSize);

	for (;;)
	{
		if (FALSE == GetFileSecurityW(backupRoot.c_str(), OWNER_SECURITY_INFORMATION,
			&buffer[0], static_cast<DWORD>(buffer.size()), &bufferSize))
		{
			if (ERROR_INSUFFICIENT_BUFFER == GetLastError())
			{
				buffer.resize(bufferSize);
				continue;
			}

			throw std::runtime_error("Could not acquire security descriptor of backup directory");
		}

		break;
	}

	SID *sid = nullptr;
	BOOL ownerDefaulted = FALSE;

	if (FALSE == GetSecurityDescriptorOwner(reinterpret_cast<SECURITY_DESCRIPTOR *>(&buffer[0]),
		reinterpret_cast<PSID *>(&sid), &ownerDefaulted))
	{
		throw std::runtime_error("Could not determine owner of backup directory");
	}

	if (FALSE == IsWellKnownSid(sid, WinLocalSystemSid))
	{
		throw std::runtime_error("Backup directory is not owned by SYSTEM");
	}

	//
	// Ensure destination directory exists
	//

	if (false == fs::exists(mullvadAppData)
		&& false == fs::create_directory(mullvadAppData))
	{
		throw std::runtime_error("Could not create destination directory during migration");
	}

	//
	// Specify files that need to be copied over
	//

	struct FileMigration
	{
		std::wstring filename;
		bool required;
	};

	const FileMigration filesToMigrate[] = {
		{ L"settings.json", true },
		{ L"account-history.json", false },
	};

	//
	// Copy and delete files
	//

	bool copyStatus = true;

	for (const auto file : filesToMigrate)
	{
		const auto from = fs::path(backupMullvadAppData).append(file.filename);
		const auto to = fs::path(mullvadAppData).append(file.filename);

		std::error_code error;

		if (fs::copy_file(from, to, fs::copy_options::overwrite_existing | fs::copy_options::skip_symlinks, error))
		{
			fs::remove(from, error);
		}
		else if (file.required)
		{
			copyStatus = false;
		}
	}

	if (false == copyStatus)
	{
		throw std::runtime_error("Failed to copy files during migration");
	}

	return MigrationStatus::Success;
}

}
