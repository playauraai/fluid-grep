@echo off
REM Benchmark: Original vs Fluid Search Mode

setlocal enabledelayedexpansion

set RG=.\target\release\rg.exe
set PATTERN=fn
set ITERATIONS=5

echo.
echo ==========================================
echo Ripgrep Benchmark: Original vs Fluid
echo ==========================================
echo.

REM Create test data
echo Creating test data...
if not exist benchmark_data mkdir benchmark_data

for /L %%i in (1,1,100) do (
    (
        echo fn main^(^) {
        echo     println!^("Hello, world!"^)^;
        echo }
        echo.
        echo fn add^(a: i32, b: i32^) -^> i32 {
        echo     a + b
        echo }
        echo.
        echo fn subtract^(a: i32, b: i32^) -^> i32 {
        echo     a - b
        echo }
        echo.
        echo fn multiply^(a: i32, b: i32^) -^> i32 {
        echo     a * b
        echo }
        echo.
        echo fn divide^(a: i32, b: i32^) -^> i32 {
        echo     if b == 0 {
        echo         panic!^("Division by zero"^)^;
        echo     }
        echo     a / b
        echo }
        echo.
        echo fn fibonacci^(n: u32^) -^> u32 {
        echo     match n {
        echo         0 =^> 0,
        echo         1 =^> 1,
        echo         _ =^> fibonacci^(n - 1^) + fibonacci^(n - 2^),
        echo     }
        echo }
        echo.
        echo fn factorial^(n: u32^) -^> u32 {
        echo     match n {
        echo         0 =^> 1,
        echo         _ =^> n * factorial^(n - 1^),
        echo     }
        echo }
    ) > benchmark_data\test_%%i.rs
)

echo Test data created (100 files)
echo.

REM Benchmark Original Mode
echo ==========================================
echo ORIGINAL MODE (Regex-based)
echo ==========================================
echo Pattern: '%PATTERN%'
echo Iterations: %ITERATIONS%
echo.

setlocal enabledelayedexpansion
set total_time=0

for /L %%i in (1,1,%ITERATIONS%) do (
    for /f "tokens=*" %%A in ('powershell -Command "Measure-Command { & '%RG%' '%PATTERN%' benchmark_data ^> $null } | Select-Object -ExpandProperty TotalMilliseconds"') do (
        set elapsed=%%A
        echo Run %%i: !elapsed!ms
        set /a total_time=!total_time! + !elapsed!
    )
)

set /a avg_original=!total_time! / %ITERATIONS%
echo Average: %avg_original%ms
echo.

REM Benchmark Fluid Mode
echo ==========================================
echo FLUID MODE (Heuristic-based)
echo ==========================================
echo Pattern: '%PATTERN%'
echo Iterations: %ITERATIONS%
echo.

set total_time=0

for /L %%i in (1,1,%ITERATIONS%) do (
    for /f "tokens=*" %%A in ('powershell -Command "Measure-Command { & '%RG%' --fluid '%PATTERN%' benchmark_data ^> $null } | Select-Object -ExpandProperty TotalMilliseconds"') do (
        set elapsed=%%A
        echo Run %%i: !elapsed!ms
        set /a total_time=!total_time! + !elapsed!
    )
)

set /a avg_fluid=!total_time! / %ITERATIONS%
echo Average: %avg_fluid%ms
echo.

REM Calculate improvement
echo ==========================================
echo RESULTS
echo ==========================================
echo Original Mode: %avg_original%ms
echo Fluid Mode:    %avg_fluid%ms

if %avg_original% gtr %avg_fluid% (
    set /a improvement=(%avg_original% - %avg_fluid%) * 100 / %avg_original%
    echo Improvement:   %improvement%% faster
) else (
    set /a difference=(%avg_fluid% - %avg_original%) * 100 / %avg_original%
    echo Difference:    %difference%% slower (expected for heuristic mode)
)

echo.
echo Cleanup...
rmdir /s /q benchmark_data
echo Done!
