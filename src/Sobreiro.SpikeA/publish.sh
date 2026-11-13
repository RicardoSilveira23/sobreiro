#!/bin/bash
# SPIKE (shell candidate A): publishes the spike into dist/spike-a/<kind> and keeps the whole build log.
#   ./publish.sh aot   Native AOT (needs the MSVC linker)
#   ./publish.sh jit   the same code as a self-contained single file on the .NET runtime, for comparison
set -u
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
kind="${1:-aot}"
# The toolkit's build step reports usage to its maker unless this is set.
export AVALONIA_TELEMETRY_OPTOUT=1
# The Build Tools' environment script runs vswhere.exe by name; its folder is not on PATH after a plain install.
export PATH="$PATH:/c/Program Files (x86)/Microsoft Visual Studio/Installer"
mkdir -p "$root/dist/spike-a-logs"
log="$root/dist/spike-a-logs/publish-$kind.log"
out="$root/dist/spike-a/$kind"
rm -rf "$out"
start=$(date +%s)
if [ "$kind" = "aot" ]; then
  dotnet publish "$here" -c Release -r win-x64 -o "$out" > "$log" 2>&1
else
  dotnet publish "$here" -c Release -r win-x64 --self-contained true -p:SpikeNoAot=true -p:PublishSingleFile=true \
    -p:IncludeNativeLibrariesForSelfExtract=true -p:DebugType=none -o "$out" > "$log" 2>&1
fi
code=$?
echo "exit $code after $(( $(date +%s) - start )) s; warnings: $(grep -c -i 'warning' "$log"); errors: $(grep -c ' error ' "$log")"
grep -i -E "warning| error " "$log" | sed -e 's/ \[.*$//' | sort | uniq -c | sort -rn | head -40
ls -la "$out"
