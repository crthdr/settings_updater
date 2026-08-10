setlocal
(set RUSTFLAGS=-Zunstable-options -Cpanic=immediate-abort -Zfmt-debug=none -Zlocation-detail=none) && cargo +nightly build --lib -Z build-std=std,panic_abort -Z build-std-features="optimize_for_size" --target x86_64-pc-windows-msvc --release
mkdir build\bin\x64
copy target\x86_64-pc-windows-msvc\release\settings_updater.dll build\bin\x64\settings_updater.asi
mkdir build\bin\x64_dx12
copy target\x86_64-pc-windows-msvc\release\settings_updater.dll build\bin\x64_dx12\settings_updater.asi
pause
endlocal