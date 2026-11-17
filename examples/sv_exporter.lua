if not util.IsBinaryModuleInstalled("exporter") then
    return
end

local serverName = "SCP Roleplay"

require("exporter")
Prometheus.StartServer("0.0.0.0:7777")
Prometheus.CreateCounter("entities", "server,classname")
Prometheus.CreateGauge("tickrate", "server")
Prometheus.CreateGauge("jobplayers", "server,job,category")

function Exporter_Stop()
   Prometheus.StopServer()
end

local Player = FindMetaTable("Player")
local Team = Player.Team
local Entity = FindMetaTable("Entity")
local GetClass = Entity.GetClass

local pendingJobs = {}
local function UpdateJobs()
	if table.IsEmpty(pendingJobs) then return end

	local teamCounts = {}
	for ply in player.Iterator() do
		local job = Team(ply)
		if pendingJobs[job] then
			teamCounts[job] = (teamCounts[job] or 0) + 1
		end
	end

	for job in pairs(pendingJobs) do
		local data = RPExtraTeams[job]
		Prometheus.SetGauge("jobplayers", {serverName, data.name, data.category}, teamCounts[job] or 0)
	end

	pendingJobs = {}
end

local modules = {
	["entities"] = {
		default = true,
		enable = function()
			for k, v in ents.Iterator() do
				Prometheus.IncreaseCounter("entities", {serverName, GetClass(v)}, 1)
			end

			hook.Add("OnEntityCreated", "Exporter_Entities", function(ent)
				Prometheus.IncreaseCounter("entities", {serverName, GetClass(ent)}, 1)
			end)

			hook.Add("EntityRemoved", "Exporter_Entities", function(ent)
				Prometheus.IncreaseCounter("entities", {serverName, GetClass(ent)}, -1)
			end)
		end,

		disable = function()
			Prometheus.Reset("entities")
			hook.Remove("OnEntityCreated", "Exporter_Entities")
			hook.Remove("EntityRemoved", "Exporter_Entities")
		end
	},

	["tickrate"] = {
		default = true,
		enable = function()
			local lastTime = RealTime()
			local lastTick = engine.TickCount()

			Prometheus.SetGauge("tickrate", {serverName}, 1 / engine.TickInterval())

			timer.Create("Exporter_Tickrate", 5, 0, function()
			    local now = RealTime()
			    local tick = engine.TickCount()

			    local dt = now - lastTime
			    local ticks = tick - lastTick

			    Prometheus.SetGauge("tickrate", {serverName}, ticks / dt)

			    lastTime = now
			    lastTick = tick
			end)
		end,

		disable = function()
			timer.Remove("Exporter_Tickrate")
		end
	},

	["jobplayers"] = {
		default = true,
		enable = function()
			local defaultTeam = GAMEMODE.DefaultTeam
			for job in ipairs(RPExtraTeams) do
				pendingJobs[job] = true
			end

			UpdateJobs()
			timer.Create("Exporter_JobPlayers", 5, 0, UpdateJobs)

			hook.Add("PlayerInitialSpawn", "Exporter_JobPlayers", function(ply)
				pendingJobs[defaultTeam] = true
			end, HOOK_MONITOR_LOW)

			hook.Add("PlayerDisconnected", "Exporter_JobPlayers", function(ply)
				pendingJobs[Team(ply)] = true
				if player.Count() == 0 then
					UpdateJobs()
				end
			end, HOOK_MONITOR_LOW)

			hook.Add("OnPlayerChangedTeam", "Exporter_JobPlayers", function(ply, old, new)
				pendingJobs[old] = true
				pendingJobs[new] = true
			end, HOOK_MONITOR_LOW)
		end,

		disable = function()
			timer.Remove("Exporter_JobPlayers")

			hook.Remove("PlayerInitialSpawn", "Exporter_JobPlayers")
			hook.Remove("PlayerDisconnected", "Exporter_JobPlayers")
			hook.Remove("OnPlayerChangedTeam", "Exporter_JobPlayers")
		end
	},

}

function Exporter_ToggleModule(moduleName)
	local mod = modules[moduleName]
	if not mod then return end

	if mod.state then
		mod.state = false
		mod.disable()
	else
		mod.state = true
		mod.enable()
	end
end

timer.Simple(0, function()
	for moduleName, data in pairs(modules) do
		if data.default then
			ServerLog("[Exporter] Enabling " .. moduleName .. "\n")
			data.state = true
			data.enable()
		else
			data.state = false
		end
	end
end)