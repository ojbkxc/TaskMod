use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceStatus {
    Unloaded,
    Loading,
    Loaded,
    Unloading,
}

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub status: ServiceStatus,
    pub ref_count: usize,
    pub last_used: std::time::Instant,
}

type ServiceId = String;
type LoadFn = Box<dyn Fn() -> Result<(), String> + Send + Sync + 'static>;
type UnloadFn = Box<dyn Fn() -> Result<(), String> + Send + Sync + 'static>;

struct ServiceDefinition {
    load_fn: LoadFn,
    unload_fn: UnloadFn,
}

struct ServiceManagerInner {
    services: HashMap<ServiceId, ServiceDefinition>,
    service_info: HashMap<ServiceId, ServiceInfo>,
}

lazy_static::lazy_static! {
    static ref SERVICE_MANAGER: Arc<Mutex<ServiceManagerInner>> = Arc::new(Mutex::new(ServiceManagerInner {
        services: HashMap::new(),
        service_info: HashMap::new(),
    }));
}

pub fn register_service(id: &str, load_fn: LoadFn, unload_fn: UnloadFn) {
    let mut inner = SERVICE_MANAGER.lock().unwrap();
    inner.services.insert(id.to_string(), ServiceDefinition { load_fn, unload_fn });
    inner.service_info.insert(id.to_string(), ServiceInfo {
        status: ServiceStatus::Unloaded,
        ref_count: 0,
        last_used: std::time::Instant::now(),
    });
}

pub fn get_service_status(id: &str) -> Option<ServiceStatus> {
    let inner = SERVICE_MANAGER.lock().unwrap();
    inner.service_info.get(id).map(|info| info.status.clone())
}

pub fn load_service(id: &str) -> Result<(), String> {
    let mut inner = SERVICE_MANAGER.lock().unwrap();
    
    let service = match inner.services.get(id) {
        Some(s) => s,
        None => return Err(format!("服务不存在: {}", id)),
    };
    
    let info = inner.service_info.get_mut(id).unwrap();
    
    if info.status == ServiceStatus::Loaded {
        info.ref_count += 1;
        info.last_used = std::time::Instant::now();
        return Ok(());
    }
    
    info.status = ServiceStatus::Loading;
    drop(inner);
    
    match (service.load_fn)() {
        Ok(_) => {
            let mut inner = SERVICE_MANAGER.lock().unwrap();
            if let Some(info) = inner.service_info.get_mut(id) {
                info.status = ServiceStatus::Loaded;
                info.ref_count = 1;
                info.last_used = std::time::Instant::now();
            }
            Ok(())
        }
        Err(e) => {
            let mut inner = SERVICE_MANAGER.lock().unwrap();
            if let Some(info) = inner.service_info.get_mut(id) {
                info.status = ServiceStatus::Unloaded;
            }
            Err(e)
        }
    }
}

pub fn unload_service(id: &str) -> Result<(), String> {
    let mut inner = SERVICE_MANAGER.lock().unwrap();
    
    let service = match inner.services.get(id) {
        Some(s) => s,
        None => return Err(format!("服务不存在: {}", id)),
    };
    
    let info = inner.service_info.get_mut(id).unwrap();
    
    if info.ref_count > 0 {
        info.ref_count -= 1;
        info.last_used = std::time::Instant::now();
        
        if info.ref_count == 0 {
            info.status = ServiceStatus::Unloading;
            drop(inner);
            
            match (service.unload_fn)() {
                Ok(_) => {
                    let mut inner = SERVICE_MANAGER.lock().unwrap();
                    if let Some(info) = inner.service_info.get_mut(id) {
                        info.status = ServiceStatus::Unloaded;
                    }
                    Ok(())
                }
                Err(e) => {
                    let mut inner = SERVICE_MANAGER.lock().unwrap();
                    if let Some(info) = inner.service_info.get_mut(id) {
                        info.status = ServiceStatus::Loaded;
                        info.ref_count = 0;
                    }
                    Err(e)
                }
            }
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}

pub fn use_service<F, T>(id: &str, f: F) -> Result<T, String>
where
    F: FnOnce() -> T,
{
    load_service(id)?;
    let result = f();
    unload_service(id)?;
    Ok(result)
}

pub async fn use_service_async<F, T>(id: &str, f: F) -> Result<T, String>
where
    F: FnOnce() -> T + Send,
{
    load_service(id)?;
    let result = f();
    unload_service(id)?;
    Ok(result)
}

pub fn get_all_service_info() -> Vec<(ServiceId, ServiceInfo)> {
    let inner = SERVICE_MANAGER.lock().unwrap();
    inner.service_info.iter()
        .map(|(id, info)| (id.clone(), info.clone()))
        .collect()
}

pub fn set_service_loaded(id: &str) {
    let mut inner = SERVICE_MANAGER.lock().unwrap();
    if let Some(info) = inner.service_info.get_mut(id) {
        info.status = ServiceStatus::Loaded;
        info.last_used = std::time::Instant::now();
    }
}

pub fn set_service_unloaded(id: &str) {
    let mut inner = SERVICE_MANAGER.lock().unwrap();
    if let Some(info) = inner.service_info.get_mut(id) {
        info.status = ServiceStatus::Unloaded;
        info.ref_count = 0;
        info.last_used = std::time::Instant::now();
    }
}