use std::{mem, slice, str};

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;

// 最小構成: HS256 共有鍵（本番は Edge から渡す/Secrets 管理を推奨）
static SECRET: &[u8] = b"super-secret-key";
static AUDIENCE: &str = "my-audience";
static ISSUER: &str = "my-issuer";

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    aud: String,
    iss: String,
}

/// Edge 側が token bytes を置くための領域を確保する。
/// - WASMはHTTPを知らない（bytesの受け渡しのみ）
/// - Edgeは検証ロジックを知らない（validate_tokenの結果のみ）
#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    // Rust側で解放されないようにリークさせ、deallocで回収する
    mem::forget(buf);
    ptr
}

/// alloc で確保した領域を解放する。
#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    unsafe {
        // capacity=len で確保した前提（Edgeが同じlenで呼ぶ）
        drop(Vec::<u8>::from_raw_parts(ptr, 0, len));
    }
}

#[repr(i32)]
pub enum AuthResult {
    Ok = 1,
    Expired = -1,
    Invalid = 0,
}

/// JWT を検証する（純粋ロジックのみ）
/// 戻り値:
///  1  = OK
///  0  = Invalid
/// -1  = Expired
#[no_mangle]
pub extern "C" fn validate_token(ptr: *const u8, len: usize) -> i32 {
    let bytes = unsafe { slice::from_raw_parts(ptr, len) };

    let token = match str::from_utf8(bytes) {
        Ok(v) => v,
        Err(_) => return AuthResult::Invalid as i32,
    };

    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&[AUDIENCE]);
    validation.set_issuer(&[ISSUER]);

    let result = decode::<Claims>(
        token,
        &DecodingKey::from_secret(SECRET),
        &validation,
    );

    match result {
        Ok(_) => AuthResult::Ok as i32,
        Err(err)
            if matches!(
                err.kind(),
                jsonwebtoken::errors::ErrorKind::ExpiredSignature
            ) =>
        {
            AuthResult::Expired as i32
        }
        Err(_) => AuthResult::Invalid as i32,
    }
}


