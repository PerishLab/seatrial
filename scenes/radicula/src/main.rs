mod proxy;

use radicula_lock::{
    Factory, Policy, Wait,
    redis::{Config, Gate, Permit},
};
use std::{
    collections::BTreeMap,
    io::{self, BufRead, Write},
    time::Duration,
};

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.get(1).map(String::as_str) == Some("proxy") {
        proxy::serve(&args[2], &args[3], &args[4]);
        return;
    }
    let millis = |index: usize| Duration::from_millis(args[index].parse().unwrap());
    let renewal = match args[5].as_str() {
        "none" => None,
        _ => Some(millis(5)),
    };
    let config = Config::new(args[1].parse().unwrap(), &args[2])
        .io(millis(3))
        .lease(millis(4), renewal)
        .slots(2);
    let factory = Factory::redis(config).unwrap();
    println!("ready {}", factory.epoch());
    io::stdout().flush().unwrap();
    let mut gates = BTreeMap::<String, Gate>::new();
    let mut permits = BTreeMap::<String, Permit>::new();
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let parts = line.split_whitespace().collect::<Vec<_>>();
        let response = execute(&factory, &mut gates, &mut permits, &parts);
        println!("{response}");
        io::stdout().flush().unwrap();
    }
}

fn execute(
    factory: &radicula_lock::redis::Factory,
    gates: &mut BTreeMap<String, Gate>,
    permits: &mut BTreeMap<String, Permit>,
    args: &[&str],
) -> String {
    match args {
        ["gate", name, "readers"] => bind(factory, gates, name, Policy::Readers),
        ["gate", name, capacity] => bind(
            factory,
            gates,
            name,
            Policy::limit(capacity.parse().unwrap()).unwrap(),
        ),
        ["take", key, id, mode] => take(gates.get(*key).unwrap(), permits, id, mode),
        ["drop", id] => match permits.remove(*id).unwrap().release() {
            Ok(()) => "released".into(),
            Err(error) => format!("error {error:?}"),
        },
        ["check", id] => match permits.get(*id).unwrap().check() {
            Ok(()) => "valid".into(),
            Err(error) => format!("error {error:?}"),
        },
        ["status", key] => {
            let status = gates.get(*key).unwrap().status();
            format!("status {} {}", status.active, status.fault.is_some())
        }
        ["guard", key, "error"] => format!(
            "result {:?}",
            gates.get(*key).unwrap().guard(|| Err::<(), _>("business"))
        ),
        ["guard", key, "panic"] => {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                gates.get(*key).unwrap().guard(|| panic!("business"))
            }));
            format!("panic {}", result.is_err())
        }
        ["macro", key] => format!("result {:?}", guarded(gates.get(*key).unwrap())),
        _ => panic!("invalid conformance command"),
    }
}

fn bind(
    factory: &radicula_lock::redis::Factory,
    gates: &mut BTreeMap<String, Gate>,
    name: &str,
    policy: Policy,
) -> String {
    match factory.gate(name, policy) {
        Ok(gate) => {
            gates.insert(name.into(), gate);
            "bound".into()
        }
        Err(error) => format!("error {error:?}"),
    }
}

#[radicula_lock::guard(gate = gate, error = |_| Err("coordination"))]
fn guarded(gate: &Gate) -> Result<usize, &'static str> {
    Ok(42)
}

fn take(gate: &Gate, permits: &mut BTreeMap<String, Permit>, id: &str, mode: &str) -> String {
    let gate = match mode {
        "shared" => gate.shared().unwrap(),
        _ => gate.clone(),
    };
    match gate.acquire(Wait::immediate()) {
        Ok(permit) => {
            let fence = permit.fence().unwrap();
            assert!(!permits.contains_key(id));
            permits.insert(id.into(), permit);
            format!("held {} {}", fence.epoch, fence.sequence)
        }
        Err(error) => format!("error {error:?}"),
    }
}
