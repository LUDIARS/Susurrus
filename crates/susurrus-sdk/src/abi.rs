//! C ABI ラッパ。 cdylib 出力 (`susurrus_sdk.dll/.so/.dylib`) で
//! Unity / Pictor / 任意 C/C++ アプリから dlopen 経由で呼べる。
//!
//! メモリモデル:
//! - 戻り値の char* / void* は SDK が malloc し、 呼び出し側が `susurrus_free`
//!   で解放する。
//! - JSON 文字列を yields する関数は UTF-8 NUL 終端。
//! - エラーは戻り値 nullptr / 負数 + `susurrus_last_error` で詳細を取得。

use crate::client::Susurrus;
use crate::types::SpatialPosition;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_float, c_int};
use std::sync::Mutex;
use tokio::runtime::Runtime;

static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    Runtime::new().expect("susurrus-sdk: failed to start tokio runtime")
});

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_error(msg: impl Into<String>) {
    let c = CString::new(msg.into()).unwrap_or_else(|_| CString::new("error").unwrap());
    LAST_ERROR.with(|e| *e.borrow_mut() = Some(c));
}

/// 直近のエラーメッセージ (UTF-8 NUL 終端)。 戻り値はライブラリ管理 (free 不要)。
/// エラーがない場合は nullptr。
#[no_mangle]
pub extern "C" fn susurrus_last_error() -> *const c_char {
    LAST_ERROR.with(|e| {
        e.borrow().as_ref().map(|c| c.as_ptr()).unwrap_or(std::ptr::null())
    })
}

/// SDK が malloc した文字列を解放する。
#[no_mangle]
pub unsafe extern "C" fn susurrus_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

/// Susurrus client ハンドル (opaque)。
pub struct SusurrusHandle {
    inner: Mutex<Susurrus>,
}

/// 新しいクライアントを作る。 endpoint が NULL なら "http://127.0.0.1:17370"。
/// 戻り値は opaque handle、 不要になったら `susurrus_destroy` で解放。
#[no_mangle]
pub unsafe extern "C" fn susurrus_create(endpoint: *const c_char) -> *mut SusurrusHandle {
    let ep = if endpoint.is_null() {
        "http://127.0.0.1:17370".to_string()
    } else {
        match CStr::from_ptr(endpoint).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => {
                set_error("endpoint is not valid UTF-8");
                return std::ptr::null_mut();
            }
        }
    };
    let h = Box::new(SusurrusHandle {
        inner: Mutex::new(Susurrus::new(ep)),
    });
    Box::into_raw(h)
}

#[no_mangle]
pub unsafe extern "C" fn susurrus_destroy(h: *mut SusurrusHandle) {
    if !h.is_null() {
        let _ = Box::from_raw(h);
    }
}

/// ping → "pong" を確認。 成功時 0、 失敗時 -1。
#[no_mangle]
pub unsafe extern "C" fn susurrus_ping(h: *const SusurrusHandle) -> c_int {
    if h.is_null() { set_error("null handle"); return -1; }
    let client = &(*h).inner;
    let res = RUNTIME.block_on(async {
        let g = client.lock().unwrap();
        g.ping().await
    });
    match res {
        Ok(_) => 0,
        Err(e) => { set_error(format!("{e}")); -1 }
    }
}

/// reply を投稿。 成功時に malloc した CString (新 reply id)、 失敗時 nullptr。
#[no_mangle]
pub unsafe extern "C" fn susurrus_send_reply(
    h: *const SusurrusHandle,
    thread_id: *const c_char,
    author: *const c_char,
    body: *const c_char,
) -> *mut c_char {
    if h.is_null() || thread_id.is_null() || author.is_null() || body.is_null() {
        set_error("null arg");
        return std::ptr::null_mut();
    }
    let tid = match CStr::from_ptr(thread_id).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => { set_error("thread_id utf8"); return std::ptr::null_mut(); }
    };
    let au = match CStr::from_ptr(author).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => { set_error("author utf8"); return std::ptr::null_mut(); }
    };
    let bo = match CStr::from_ptr(body).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => { set_error("body utf8"); return std::ptr::null_mut(); }
    };
    let client = &(*h).inner;
    let res = RUNTIME.block_on(async {
        let g = client.lock().unwrap();
        g.send_reply(&tid, &au, &bo).await
    });
    match res {
        Ok(id) => CString::new(id).map(|c| c.into_raw()).unwrap_or(std::ptr::null_mut()),
        Err(e) => { set_error(format!("{e}")); std::ptr::null_mut() }
    }
}

/// position を報告 (Spatial)。 成功時 0、 失敗時 -1。
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn susurrus_report_position(
    h: *const SusurrusHandle,
    user: *const c_char,
    forum_id: *const c_char,
    x: c_float, y: c_float, z: c_float,
    qx: c_float, qy: c_float, qz: c_float, qw: c_float,
) -> c_int {
    if h.is_null() || user.is_null() || forum_id.is_null() {
        set_error("null arg"); return -1;
    }
    let user = match CStr::from_ptr(user).to_str() { Ok(s) => s.to_string(), _ => { set_error("user utf8"); return -1; } };
    let forum = match CStr::from_ptr(forum_id).to_str() { Ok(s) => s.to_string(), _ => { set_error("forum utf8"); return -1; } };
    let pos = SpatialPosition { x, y, z, qx, qy, qz, qw };
    let client = &(*h).inner;
    let res = RUNTIME.block_on(async {
        let g = client.lock().unwrap();
        g.report_position(&user, &forum, pos).await
    });
    match res {
        Ok(()) => 0,
        Err(e) => { set_error(format!("{e}")); -1 }
    }
}
