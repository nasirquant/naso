@echo off
set PATH=C:\Program Files\CMake\bin;%PATH%
set TEMP=C:\Temp
set TMP=C:\Temp
set CARGO_TEMP_DIR=C:\Temp
set CARGO_TARGET_DIR=C:\Users\E\.hermes\projects\Naso\target
if not exist C:\Temp mkdir C:\Temp
cd /d C:\Users\E\.hermes\projects\Naso
cargo check --no-default-features 2>&1