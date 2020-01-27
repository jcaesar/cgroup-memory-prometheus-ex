# CGroups 

Exports a single vector of metrics for the exclusive memory use per cgroup (i.e. excluding the subgroups).
This makes for a neat stacking graph in Grafana.

I really hope I understood cgroups right and that makes sense.

```
# HELP cgroup_memory_bytes CGroup exclusive memory use
# TYPE cgroup_memory_bytes gauge
cgroup_memory_bytes{path="/"} 7991296
cgroup_memory_bytes{path="/init.scope"} 3821568
cgroup_memory_bytes{path="/system.slice"} 76275712
cgroup_memory_bytes{path="/system.slice/cgroup-memory-prometheus-exporter.service"} 1773568
cgroup_memory_bytes{path="/system.slice/cronie.service"} 228188160
cgroup_memory_bytes{path="/system.slice/dbus.service"} 1662976
cgroup_memory_bytes{path="/system.slice/dhcpcd.service"} 3055616
```

I originally wanted cadvisor, but for finding memory troubles on a small server that isn't even running docker… this is the cuter hack.
