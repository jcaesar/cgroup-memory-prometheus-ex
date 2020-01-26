use std::net::SocketAddr;
use structopt::StructOpt;
use prometheus::{
	__register_gauge_vec,
	opts,
	register_int_gauge_vec,
	IntGauge,
};
use prometheus_exporter::{
	FinishedUpdate,
	PrometheusExporter,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::cmp::min;
use std::collections::HashMap;
use cgroups::{Hierarchy, Controller, Cgroup};
use std::convert::TryInto;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(StructOpt)]
#[structopt(author = env!("CARGO_PKG_AUTHORS"), about = env!("CARGO_PKG_DESCRIPTION"))]
struct Opts {
	/// Listen address/port
	#[structopt(short = "l", long = "listen", default_value = "[::]:9134")]
	listen: SocketAddr,
}

struct StatRes {
	pidcount: usize,
	mem: u64,
}

type ScanRes = HashMap<Box<PathBuf>, u64>;

struct ScanBase<'a> {
	base: Box<PathBuf>,
	hier: &'a dyn Hierarchy,
}

impl<'a> ScanBase<'a> {
	fn new(hier: &'a dyn Hierarchy) -> ScanBase<'a> {
		let rootgroup = hier.root_control_group();
		let mem = rootgroup.controller_of::<cgroups::memory::MemController>().unwrap();
		ScanBase {
			base: Box::new(mem.path().to_owned()),
			hier: hier,
		}
	}
	fn stat(&self, group: &Path, result: &mut ScanRes) -> Result<StatRes> {
		let cg = Cgroup::load(self.hier, group);
		let mem = cg
			.controller_of::<cgroups::memory::MemController>()
			.ok_or(failure::err_msg("Not a ss"))?;
		let mut totalpids = cg.tasks().len();
		let totaluse = mem.memory_stat().usage_in_bytes;
		let mut memuse = totaluse;
		for entry in fs::read_dir(mem.path())? { for entry in entry {
			let path = entry.path();
			if path.is_dir() {
				if let Ok(StatRes { pidcount, mem }) = self.stat(&path, result) {
					totalpids += pidcount;
					// TODO: warn if the min has any kind of effect
					memuse -= min(mem, memuse);
				}
			}
		}}
		if totalpids > 0 && memuse > 0 {
			let diff = pathdiff::diff_paths(mem.path(), &self.base)
				.ok_or(failure::err_msg("Can't get relative path"))?;
			//println!("{}:{}", memuse, diff.display());
			result.insert(Box::new(diff), memuse);

		}
		Ok(StatRes { pidcount: totalpids, mem: memuse })
	}
}

macro_rules! path_to_label {
    ($expression:expr) => {
        &[&format!("/{}", $expression.display())]
    };
}


fn main() -> Result<()> {
	let hier = cgroups::hierarchies::V1::new();
	let opts: Opts = Opts::from_args();
	let scan = ScanBase::new(&hier);
	let memmet: prometheus::IntGaugeVec = register_int_gauge_vec!("cgroup_memory_bytes", "CGroup exclusive memory use", &["path"]).unwrap();
	let mut mets: HashMap<Box<PathBuf>, IntGauge> = HashMap::new();
	let (request_receiver, finished_sender) = PrometheusExporter::run_and_notify(opts.listen);
	loop {
		request_receiver.recv().unwrap();
		let mut res = ScanRes::new();
		scan.stat(&scan.base, &mut res).ok(); // TODO: want to send a fail
		for (path, _) in res.iter() {
			if !mets.contains_key(path) {
				if let Ok(met) = memmet.get_metric_with_label_values(path_to_label!(path)) {
					mets.insert(path.clone(), met);
				}
			}
		}
		mets.retain(|path, met| {
			let keep = res.get(path)
                .and_then(|res| (*res).try_into().ok())
                .map(|res| met.set(res))
			    .is_some();
            if !keep {
                memmet.remove_label_values(path_to_label!(path)).ok();
            };
            return keep
		});
		finished_sender.send(FinishedUpdate).unwrap();
		mets.shrink_to_fit();
	}
}
