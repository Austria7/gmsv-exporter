use rglua::lua::{LuaState, lua_next, lua_pop, lua_pushnil, lua_tonumber, lua_tostring};
use rglua::prelude::lua_type;
use std::collections::HashMap;
use std::ffi::CStr;

pub const LUA_TNIL: i32 = 0;
pub const LUA_TBOOLEAN: i32 = 1;
pub const LUA_TNUMBER: i32 = 3;
pub const LUA_TSTRING: i32 = 4;
pub const LUA_TTABLE: i32 = 5;
pub const LUA_TFUNCTION: i32 = 6;
pub const LUA_TUSERDATA: i32 = 7;
pub const LUA_TTHREAD: i32 = 8;

pub trait FromLua: Sized {
    unsafe fn from_lua(l: LuaState, index: i32) -> Self;
}

impl FromLua for String {
    unsafe fn from_lua(l: LuaState, index: i32) -> Self {
        match lua_type(l, index) {
            LUA_TSTRING => {
                let ptr = lua_tostring(l, index);
                if ptr.is_null() {
                    return String::new();
                }

                CStr::from_ptr(ptr).to_str().unwrap().to_owned()
            }

            LUA_TNUMBER => lua_tonumber(l, index).to_string(),

            _ => "Unknown".to_owned(),
        }
    }
}

impl FromLua for i64 {
    unsafe fn from_lua(l: LuaState, index: i32) -> Self {
        match lua_type(l, index) {
            LUA_TNUMBER => lua_tonumber(l, index) as i64,
            _ => -1,
        }
    }
}

pub unsafe fn get_table(l: LuaState, index: i32) -> HashMap<i64, String> {
    let mut map = HashMap::new();

    if lua_type(l, index) != LUA_TTABLE {
        return map;
    }

    lua_pushnil(l);
    unsafe {
        while lua_next(l, index) != 0 {
            let key = i64::from_lua(l, -2);
            let value = String::from_lua(l, -1);
            map.insert(key, value);
            lua_pop(l, 1);
        }
    }

    map
}
