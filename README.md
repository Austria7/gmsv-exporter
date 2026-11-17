# gmsv-exporter

![Stars + License](https://shieldcn.dev/group/github/license/Austria7/gmsv_exporter+github/stars/Austria7/gmsv_exporter+github/forks/Austria7/gmsv_exporter.svg?variant=branded)

Exposes a [Prometheus](https://prometheus.io/) metrics API to Garry's Mod Lua scripts.

Create counters and gauges from Lua, expose them on a local HTTP endpoint, and scrape them with any Prometheus instance.

## Usage

Place the dll file in the `garrysmod/lua/bin/` directory on the Server:

```
garrysmod/lua/bin/gmsv_exporter_linux.dll
```

Then create a server-side Lua script (e.g., `addons/exporter/lua/autorun/sv_exporter.lua`):

```lua
if not util.IsBinaryModuleInstalled("exporter") then
    return
end

require("exporter")
Prometheus.StartServer("0.0.0.0:9180")
```

A full example covering all Lua API functions can be found under [`examples/`](examples/).

## Contributing

Contributions are welcome. Open an issue or submit a pull request for bug fixes, features, or documentation
improvements.