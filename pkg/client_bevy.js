
let wasm;

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });

cachedTextDecoder.decode();

let cachegetUint8Memory0 = null;
function getUint8Memory0() {
    if (cachegetUint8Memory0 === null || cachegetUint8Memory0.buffer !== wasm.memory.buffer) {
        cachegetUint8Memory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachegetUint8Memory0;
}

function getStringFromWasm0(ptr, len) {
    return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

const heap = new Array(32).fill(undefined);

heap.push(undefined, null, true, false);

let heap_next = heap.length;

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    if (typeof(heap_next) !== 'number') throw new Error('corrupt heap');

    heap[idx] = obj;
    return idx;
}

function getObject(idx) { return heap[idx]; }

function dropObject(idx) {
    if (idx < 36) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

function _assertBoolean(n) {
    if (typeof(n) !== 'boolean') {
        throw new Error('expected a boolean argument');
    }
}

let WASM_VECTOR_LEN = 0;

let cachedTextEncoder = new TextEncoder('utf-8');

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

function passStringToWasm0(arg, malloc, realloc) {

    if (typeof(arg) !== 'string') throw new Error('expected a string argument');

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length);
        getUint8Memory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len);

    const mem = getUint8Memory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3);
        const view = getUint8Memory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);
        if (ret.read !== arg.length) throw new Error('failed to pass whole string');
        offset += ret.written;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

let cachegetInt32Memory0 = null;
function getInt32Memory0() {
    if (cachegetInt32Memory0 === null || cachegetInt32Memory0.buffer !== wasm.memory.buffer) {
        cachegetInt32Memory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachegetInt32Memory0;
}

function _assertNum(n) {
    if (typeof(n) !== 'number') throw new Error('expected a number argument');
}

let cachegetFloat64Memory0 = null;
function getFloat64Memory0() {
    if (cachegetFloat64Memory0 === null || cachegetFloat64Memory0.buffer !== wasm.memory.buffer) {
        cachegetFloat64Memory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachegetFloat64Memory0;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) {
                wasm.__wbindgen_export_2.get(state.dtor)(a, state.b);

            } else {
                state.a = a;
            }
        }
    };
    real.original = state;

    return real;
}

function logError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        let error = (function () {
            try {
                return e instanceof Error ? `${e.message}\n\nStack:\n${e.stack}` : e.toString();
            } catch(_) {
                return "<failed to stringify thrown value>";
            }
        }());
        console.error("wasm-bindgen: imported JS function that was not marked as `catch` threw an error:", error);
        throw e;
    }
}
function __wbg_adapter_30(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hff68f9f867485762(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_33(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h05fca28e4e626ef4(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_36(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hc5fa7b3183b8637c(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_39(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h2c451f08d3df5645(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_42(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h5ec808ed27ec2b57(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_45(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h5de17ebd04f74e8c(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_48(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h25ef82a65acbe2b3(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_51(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hac459c394a35a29a(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_54(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__he78e6af213532a91(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_57(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__heb3db0f9139b7f13(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_60(arg0, arg1) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hb1999c5520ff0734(arg0, arg1);
}

function __wbg_adapter_63(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h033657b7c2eb1f00(arg0, arg1, addHeapObject(arg2));
}

/**
*/
export function run() {
    wasm.run();
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_exn_store(addHeapObject(e));
    }
}

function getArrayU8FromWasm0(ptr, len) {
    return getUint8Memory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachegetFloat32Memory0 = null;
function getFloat32Memory0() {
    if (cachegetFloat32Memory0 === null || cachegetFloat32Memory0.buffer !== wasm.memory.buffer) {
        cachegetFloat32Memory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachegetFloat32Memory0;
}

function getArrayF32FromWasm0(ptr, len) {
    return getFloat32Memory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayI32FromWasm0(ptr, len) {
    return getInt32Memory0().subarray(ptr / 4, ptr / 4 + len);
}

let cachegetUint32Memory0 = null;
function getUint32Memory0() {
    if (cachegetUint32Memory0 === null || cachegetUint32Memory0.buffer !== wasm.memory.buffer) {
        cachegetUint32Memory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachegetUint32Memory0;
}

function getArrayU32FromWasm0(ptr, len) {
    return getUint32Memory0().subarray(ptr / 4, ptr / 4 + len);
}

async function load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);

    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };

        } else {
            return instance;
        }
    }
}

async function init(input) {
    if (typeof input === 'undefined') {
        input = new URL('client_bevy_bg.wasm', import.meta.url);
    }
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        var ret = getStringFromWasm0(arg0, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_cb_drop = function(arg0) {
        const obj = takeObject(arg0).original;
        if (obj.cnt-- == 1) {
            obj.a = 0;
            return true;
        }
        var ret = false;
        _assertBoolean(ret);
        return ret;
    };
    imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        var ret = typeof(obj) === 'string' ? obj : undefined;
        var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    };
    imports.wbg.__wbindgen_is_string = function(arg0) {
        var ret = typeof(getObject(arg0)) === 'string';
        _assertBoolean(ret);
        return ret;
    };
    imports.wbg.__wbindgen_object_clone_ref = function(arg0) {
        var ret = getObject(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        var ret = typeof(obj) === 'number' ? obj : undefined;
        if (!isLikeNone(ret)) {
            _assertNum(ret);
        }
        getFloat64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? 0 : ret;
        getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
    };
    imports.wbg.__wbindgen_boolean_get = function(arg0) {
        const v = getObject(arg0);
        var ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
        _assertNum(ret);
        return ret;
    };
    imports.wbg.__wbindgen_number_new = function(arg0) {
        var ret = arg0;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_mark_55c5ecfe509deb6a = function() { return logError(function (arg0, arg1) {
        performance.mark(getStringFromWasm0(arg0, arg1));
    }, arguments) };
    imports.wbg.__wbg_measure_608563106d70d48a = function() { return logError(function (arg0, arg1, arg2, arg3) {
        performance.measure(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3));
    }, arguments) };
    imports.wbg.__wbg_log_b7dcb3facc73166d = function() { return logError(function (arg0, arg1) {
        console.log(getStringFromWasm0(arg0, arg1));
    }, arguments) };
    imports.wbg.__wbg_log_96295d68ab8338df = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
        console.log(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3), getStringFromWasm0(arg4, arg5), getStringFromWasm0(arg6, arg7));
    }, arguments) };
    imports.wbg.__wbg_error_09919627ac0992f5 = function() { return logError(function (arg0, arg1) {
        try {
            console.error(getStringFromWasm0(arg0, arg1));
        } finally {
            wasm.__wbindgen_free(arg0, arg1);
        }
    }, arguments) };
    imports.wbg.__wbg_new_693216e109162396 = function() { return logError(function () {
        var ret = new Error();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_stack_0ddaca5d1abfb52f = function() { return logError(function (arg0, arg1) {
        var ret = getObject(arg1).stack;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    }, arguments) };
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbg_msCrypto_a2cdb043d2bfe57f = function() { return logError(function (arg0) {
        var ret = getObject(arg0).msCrypto;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_crypto_98fc271021c7d2ad = function() { return logError(function (arg0) {
        var ret = getObject(arg0).crypto;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getRandomValues_98117e9a7e993920 = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).getRandomValues(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_modulerequire_3440a4bcf44437db = function() { return handleError(function (arg0, arg1) {
        var ret = module.require(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_randomFillSync_64cc7d048f228ca8 = function() { return handleError(function (arg0, arg1, arg2) {
        getObject(arg0).randomFillSync(getArrayU8FromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_process_2f24d6544ea7b200 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).process;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_versions_6164651e75405d4a = function() { return logError(function (arg0) {
        var ret = getObject(arg0).versions;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_node_4b517d861cbcb3bc = function() { return logError(function (arg0) {
        var ret = getObject(arg0).node;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbindgen_is_object = function(arg0) {
        const val = getObject(arg0);
        var ret = typeof(val) === 'object' && val !== null;
        _assertBoolean(ret);
        return ret;
    };
    imports.wbg.__wbg_instanceof_WebGl2RenderingContext_56ad96bfac3f5531 = function() { return logError(function (arg0) {
        var ret = getObject(arg0) instanceof WebGL2RenderingContext;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_beginQuery_6eca464c63134897 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).beginQuery(arg1 >>> 0, getObject(arg2));
    }, arguments) };
    imports.wbg.__wbg_bindBufferRange_b859f0afc6778c43 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).bindBufferRange(arg1 >>> 0, arg2 >>> 0, getObject(arg3), arg4, arg5);
    }, arguments) };
    imports.wbg.__wbg_bindSampler_ad3c4508c8af93e2 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).bindSampler(arg1 >>> 0, getObject(arg2));
    }, arguments) };
    imports.wbg.__wbg_bindVertexArray_52b8b2f5fd93d81d = function() { return logError(function (arg0, arg1) {
        getObject(arg0).bindVertexArray(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_blitFramebuffer_d6dafd082e3b1266 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
        getObject(arg0).blitFramebuffer(arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_bufferData_43e443f819c8a238 = function() { return logError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_bufferData_bba22fbe5dd1f1d6 = function() { return logError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).bufferData(arg1 >>> 0, getObject(arg2), arg3 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_bufferSubData_27cef7a644910dfc = function() { return logError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).bufferSubData(arg1 >>> 0, arg2, getObject(arg3));
    }, arguments) };
    imports.wbg.__wbg_clearBufferfv_670d0bcaec8aa253 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).clearBufferfv(arg1 >>> 0, arg2, getArrayF32FromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_clearBufferiv_2bfc8565c8e26057 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).clearBufferiv(arg1 >>> 0, arg2, getArrayI32FromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_clearBufferuiv_d5891cc30a9d5453 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).clearBufferuiv(arg1 >>> 0, arg2, getArrayU32FromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_compressedTexSubImage2D_f004f3c7a7718bd0 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
        getObject(arg0).compressedTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8, arg9);
    }, arguments) };
    imports.wbg.__wbg_compressedTexSubImage2D_0142b826d7709c5b = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
        getObject(arg0).compressedTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, getObject(arg8));
    }, arguments) };
    imports.wbg.__wbg_compressedTexSubImage3D_e2a535df2aa9277d = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
        getObject(arg0).compressedTexSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10, arg11);
    }, arguments) };
    imports.wbg.__wbg_compressedTexSubImage3D_6f37cafcbb5b7101 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
        getObject(arg0).compressedTexSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, getObject(arg10));
    }, arguments) };
    imports.wbg.__wbg_copyBufferSubData_bb493bd9f4e48cae = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).copyBufferSubData(arg1 >>> 0, arg2 >>> 0, arg3, arg4, arg5);
    }, arguments) };
    imports.wbg.__wbg_copyTexSubImage3D_b5b81204a7dbcd2e = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
        getObject(arg0).copyTexSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9);
    }, arguments) };
    imports.wbg.__wbg_createSampler_b48a49fe620eb606 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).createSampler();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createVertexArray_d59135c0a43c410b = function() { return logError(function (arg0) {
        var ret = getObject(arg0).createVertexArray();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_deleteQuery_b1f16f0fd43c084f = function() { return logError(function (arg0, arg1) {
        getObject(arg0).deleteQuery(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_deleteSampler_8955191e3f575f47 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).deleteSampler(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_deleteSync_404eb77fed3f78e1 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).deleteSync(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_deleteVertexArray_385874f9e1499a3f = function() { return logError(function (arg0, arg1) {
        getObject(arg0).deleteVertexArray(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_drawArraysInstanced_a302763ee5a90ec9 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).drawArraysInstanced(arg1 >>> 0, arg2, arg3, arg4);
    }, arguments) };
    imports.wbg.__wbg_drawBuffers_5b889400d53a3536 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).drawBuffers(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_drawElementsInstanced_c138e56b91de9ba4 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).drawElementsInstanced(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
    }, arguments) };
    imports.wbg.__wbg_endQuery_a0fba8426ccb2bb0 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).endQuery(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_fenceSync_9b45746320921f27 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).fenceSync(arg1 >>> 0, arg2 >>> 0);
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_framebufferTextureLayer_1559a5c26e04674d = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).framebufferTextureLayer(arg1 >>> 0, arg2 >>> 0, getObject(arg3), arg4, arg5);
    }, arguments) };
    imports.wbg.__wbg_getBufferSubData_550ae28c0cd409c2 = function() { return logError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).getBufferSubData(arg1 >>> 0, arg2, getObject(arg3));
    }, arguments) };
    imports.wbg.__wbg_getQueryParameter_4173d9d31586a42c = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).getQueryParameter(getObject(arg1), arg2 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getSyncParameter_3b6731a08678114f = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).getSyncParameter(getObject(arg1), arg2 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getUniformBlockIndex_62a27116db1de46b = function() { return logError(function (arg0, arg1, arg2, arg3) {
        var ret = getObject(arg0).getUniformBlockIndex(getObject(arg1), getStringFromWasm0(arg2, arg3));
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_invalidateFramebuffer_3d3baaa6aad97a5c = function() { return handleError(function (arg0, arg1, arg2) {
        getObject(arg0).invalidateFramebuffer(arg1 >>> 0, getObject(arg2));
    }, arguments) };
    imports.wbg.__wbg_readBuffer_2da10a98c000e52d = function() { return logError(function (arg0, arg1) {
        getObject(arg0).readBuffer(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_readPixels_b28af7a10ae9c5f9 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
        getObject(arg0).readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, arg7 === 0 ? undefined : getArrayU8FromWasm0(arg7, arg8));
    }, arguments) };
    imports.wbg.__wbg_renderbufferStorageMultisample_41916a754c02df64 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).renderbufferStorageMultisample(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
    }, arguments) };
    imports.wbg.__wbg_samplerParameterf_1108ed614c95720c = function() { return logError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).samplerParameterf(getObject(arg1), arg2 >>> 0, arg3);
    }, arguments) };
    imports.wbg.__wbg_samplerParameteri_93877d3d11258844 = function() { return logError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).samplerParameteri(getObject(arg1), arg2 >>> 0, arg3);
    }, arguments) };
    imports.wbg.__wbg_texStorage2D_46acd0abc60b3422 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).texStorage2D(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
    }, arguments) };
    imports.wbg.__wbg_texStorage3D_23aee9c71c0f5f73 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
        getObject(arg0).texStorage3D(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5, arg6);
    }, arguments) };
    imports.wbg.__wbg_texSubImage2D_1fadbe96ecec5229 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
        getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
    }, arguments) };
    imports.wbg.__wbg_texSubImage2D_eee3ea60432466ce = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
        getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
    }, arguments) };
    imports.wbg.__wbg_texSubImage3D_2f5675ef74e4d01e = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
        getObject(arg0).texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, arg11);
    }, arguments) };
    imports.wbg.__wbg_texSubImage3D_1256b24d4ceb2d6b = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
        getObject(arg0).texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, getObject(arg11));
    }, arguments) };
    imports.wbg.__wbg_uniformBlockBinding_0babeb243604c06c = function() { return logError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).uniformBlockBinding(getObject(arg1), arg2 >>> 0, arg3 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_vertexAttribDivisor_8d11db24ac277254 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).vertexAttribDivisor(arg1 >>> 0, arg2 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_vertexAttribIPointer_73546947b11b6330 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).vertexAttribIPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
    }, arguments) };
    imports.wbg.__wbg_activeTexture_0092956fa2eefd8c = function() { return logError(function (arg0, arg1) {
        getObject(arg0).activeTexture(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_attachShader_7faccaa7b5ac28a6 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).attachShader(getObject(arg1), getObject(arg2));
    }, arguments) };
    imports.wbg.__wbg_bindBuffer_4ece833dd10cac2f = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).bindBuffer(arg1 >>> 0, getObject(arg2));
    }, arguments) };
    imports.wbg.__wbg_bindFramebuffer_48c4bf8ff82bf7e9 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).bindFramebuffer(arg1 >>> 0, getObject(arg2));
    }, arguments) };
    imports.wbg.__wbg_bindRenderbuffer_4a9ad1cf80580397 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).bindRenderbuffer(arg1 >>> 0, getObject(arg2));
    }, arguments) };
    imports.wbg.__wbg_bindTexture_9d8ed0fcd83eb0a9 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).bindTexture(arg1 >>> 0, getObject(arg2));
    }, arguments) };
    imports.wbg.__wbg_blendColor_c23d0906bee11d20 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).blendColor(arg1, arg2, arg3, arg4);
    }, arguments) };
    imports.wbg.__wbg_blendEquation_8f565ae0c184b00c = function() { return logError(function (arg0, arg1) {
        getObject(arg0).blendEquation(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_blendEquationSeparate_ab8dfb8eca834516 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_blendFunc_b254bb91838df1dd = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).blendFunc(arg1 >>> 0, arg2 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_blendFuncSeparate_1d03d2ee0347dd73 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_colorMask_a183552692085d61 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).colorMask(arg1 !== 0, arg2 !== 0, arg3 !== 0, arg4 !== 0);
    }, arguments) };
    imports.wbg.__wbg_compileShader_dd66d66a5a6481f3 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).compileShader(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_copyTexSubImage2D_5710e2e22dffa4f2 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
        getObject(arg0).copyTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8);
    }, arguments) };
    imports.wbg.__wbg_createBuffer_5c5caa16032a81b7 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).createBuffer();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createFramebuffer_9818fc04b4a38c18 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).createFramebuffer();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createProgram_32d01a55e144b9fc = function() { return logError(function (arg0) {
        var ret = getObject(arg0).createProgram();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createRenderbuffer_d1a55ec78e26295b = function() { return logError(function (arg0) {
        var ret = getObject(arg0).createRenderbuffer();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createShader_6e8eed55567fe1a6 = function() { return logError(function (arg0, arg1) {
        var ret = getObject(arg0).createShader(arg1 >>> 0);
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createTexture_8f31e7386e22fc37 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).createTexture();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_cullFace_811ddac8b7ea5416 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).cullFace(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_deleteBuffer_de80b51d8166fddb = function() { return logError(function (arg0, arg1) {
        getObject(arg0).deleteBuffer(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_deleteFramebuffer_5f58ccb548438c57 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).deleteFramebuffer(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_deleteProgram_3ec3c43f2cddde7f = function() { return logError(function (arg0, arg1) {
        getObject(arg0).deleteProgram(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_deleteRenderbuffer_6372146d4689793e = function() { return logError(function (arg0, arg1) {
        getObject(arg0).deleteRenderbuffer(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_deleteShader_b4e32582cfe4e771 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).deleteShader(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_deleteTexture_a0632c71429795ac = function() { return logError(function (arg0, arg1) {
        getObject(arg0).deleteTexture(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_depthFunc_63af2cb1edcba36d = function() { return logError(function (arg0, arg1) {
        getObject(arg0).depthFunc(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_depthMask_d79eec8e156a2cfb = function() { return logError(function (arg0, arg1) {
        getObject(arg0).depthMask(arg1 !== 0);
    }, arguments) };
    imports.wbg.__wbg_depthRange_2691ba0cbdc681cb = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).depthRange(arg1, arg2);
    }, arguments) };
    imports.wbg.__wbg_disable_b05e075ae54fa448 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).disable(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_disableVertexAttribArray_748bc5b21abcb0f4 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).disableVertexAttribArray(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_drawArrays_5db2f4e6291f7fb2 = function() { return logError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).drawArrays(arg1 >>> 0, arg2, arg3);
    }, arguments) };
    imports.wbg.__wbg_drawElements_a41bb53d39cd6297 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).drawElements(arg1 >>> 0, arg2, arg3 >>> 0, arg4);
    }, arguments) };
    imports.wbg.__wbg_enable_766e546395da5a5d = function() { return logError(function (arg0, arg1) {
        getObject(arg0).enable(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_enableVertexAttribArray_91da8d3cbe0c2bbd = function() { return logError(function (arg0, arg1) {
        getObject(arg0).enableVertexAttribArray(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_framebufferRenderbuffer_9d619e8bf8f2aeb6 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).framebufferRenderbuffer(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, getObject(arg4));
    }, arguments) };
    imports.wbg.__wbg_framebufferTexture2D_3da41a7f38e2c523 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).framebufferTexture2D(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, getObject(arg4), arg5);
    }, arguments) };
    imports.wbg.__wbg_frontFace_0b592d7c70e6473b = function() { return logError(function (arg0, arg1) {
        getObject(arg0).frontFace(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_getActiveUniform_d2105fe949262ffa = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).getActiveUniform(getObject(arg1), arg2 >>> 0);
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getExtension_73bff3c015bacd4a = function() { return handleError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).getExtension(getStringFromWasm0(arg1, arg2));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getParameter_e3aea13dd0a2904d = function() { return handleError(function (arg0, arg1) {
        var ret = getObject(arg0).getParameter(arg1 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getProgramInfoLog_18c849a5fa54e7b1 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg1).getProgramInfoLog(getObject(arg2));
        var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    }, arguments) };
    imports.wbg.__wbg_getProgramParameter_80edd3cfbcf7cf1d = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).getProgramParameter(getObject(arg1), arg2 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getShaderInfoLog_ba1de20c14b6fb63 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg1).getShaderInfoLog(getObject(arg2));
        var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    }, arguments) };
    imports.wbg.__wbg_getShaderParameter_264d9ab5c13ece4d = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).getShaderParameter(getObject(arg1), arg2 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getSupportedExtensions_ba08ce7d99f03cd1 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).getSupportedExtensions();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getUniformLocation_77b2d89291f84289 = function() { return logError(function (arg0, arg1, arg2, arg3) {
        var ret = getObject(arg0).getUniformLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_linkProgram_b84796e37364e5c9 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).linkProgram(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_pixelStorei_a9b9b42ef01616b2 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).pixelStorei(arg1 >>> 0, arg2);
    }, arguments) };
    imports.wbg.__wbg_polygonOffset_dcc750a9ce298efc = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).polygonOffset(arg1, arg2);
    }, arguments) };
    imports.wbg.__wbg_renderbufferStorage_6a62bdbe7df6b7ed = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).renderbufferStorage(arg1 >>> 0, arg2 >>> 0, arg3, arg4);
    }, arguments) };
    imports.wbg.__wbg_scissor_5802aaee71f2eb0e = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).scissor(arg1, arg2, arg3, arg4);
    }, arguments) };
    imports.wbg.__wbg_shaderSource_18f45f93c05a8311 = function() { return logError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).shaderSource(getObject(arg1), getStringFromWasm0(arg2, arg3));
    }, arguments) };
    imports.wbg.__wbg_stencilFuncSeparate_e0ff3858138439e3 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).stencilFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3, arg4 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_stencilMask_251b475187dff4b2 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).stencilMask(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_stencilMaskSeparate_25d10c84304047bb = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).stencilMaskSeparate(arg1 >>> 0, arg2 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_stencilOpSeparate_4533ba8d80fd712a = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).stencilOpSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_texParameteri_c54aab65b2f8cf6d = function() { return logError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
    }, arguments) };
    imports.wbg.__wbg_uniform1i_e287345af4468e22 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).uniform1i(getObject(arg1), arg2);
    }, arguments) };
    imports.wbg.__wbg_uniform4f_e82c1eabdf1458fe = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).uniform4f(getObject(arg1), arg2, arg3, arg4, arg5);
    }, arguments) };
    imports.wbg.__wbg_useProgram_c2fdf4a953d1128a = function() { return logError(function (arg0, arg1) {
        getObject(arg0).useProgram(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_vertexAttribPointer_76d558694fe81cd7 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
        getObject(arg0).vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
    }, arguments) };
    imports.wbg.__wbg_viewport_da0901eee69b9909 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).viewport(arg1, arg2, arg3, arg4);
    }, arguments) };
    imports.wbg.__wbg_instanceof_Window_c4b70662a0d2c5ec = function() { return logError(function (arg0) {
        var ret = getObject(arg0) instanceof Window;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_document_1c64944725c0d81d = function() { return logError(function (arg0) {
        var ret = getObject(arg0).document;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_innerWidth_ef25c730fca132cf = function() { return handleError(function (arg0) {
        var ret = getObject(arg0).innerWidth;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_innerHeight_1b1217a63a77bf61 = function() { return handleError(function (arg0) {
        var ret = getObject(arg0).innerHeight;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_devicePixelRatio_d8c3852bb37f76bf = function() { return logError(function (arg0) {
        var ret = getObject(arg0).devicePixelRatio;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_cancelAnimationFrame_5f2a1e987a6de100 = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).cancelAnimationFrame(arg1);
    }, arguments) };
    imports.wbg.__wbg_matchMedia_b95c474c6db67a60 = function() { return handleError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).matchMedia(getStringFromWasm0(arg1, arg2));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_open_448f550991e8f5fb = function() { return handleError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).open(getStringFromWasm0(arg1, arg2));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_requestAnimationFrame_71638ca922068239 = function() { return handleError(function (arg0, arg1) {
        var ret = getObject(arg0).requestAnimationFrame(getObject(arg1));
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_get_001a5e5e0ebf9911 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0)[getStringFromWasm0(arg1, arg2)];
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_clearTimeout_2c1ba0016d8bca41 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).clearTimeout(arg1);
    }, arguments) };
    imports.wbg.__wbg_fetch_3a636c71a7d400b0 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).fetch(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_setTimeout_df66d951b1726b78 = function() { return handleError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).setTimeout(getObject(arg1), arg2);
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_data_9e55e7d79ab13ef1 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).data;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_size_c460d27e31aa548d = function() { return logError(function (arg0) {
        var ret = getObject(arg0).size;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_type_723f5f330589b6a8 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).type;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_name_cca16bd39fdf3617 = function() { return logError(function (arg0, arg1) {
        var ret = getObject(arg1).name;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    }, arguments) };
    imports.wbg.__wbg_instanceof_HtmlCanvasElement_25d964a0dde6717e = function() { return logError(function (arg0) {
        var ret = getObject(arg0) instanceof HTMLCanvasElement;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_width_555f63ab09ba7d3f = function() { return logError(function (arg0) {
        var ret = getObject(arg0).width;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_setwidth_c1a7061891b71f25 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).width = arg1 >>> 0;
    }, arguments) };
    imports.wbg.__wbg_height_7153faec70fbaf7b = function() { return logError(function (arg0) {
        var ret = getObject(arg0).height;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_setheight_88894b05710ff752 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).height = arg1 >>> 0;
    }, arguments) };
    imports.wbg.__wbg_getContext_3e21e21280a332fc = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        var ret = getObject(arg0).getContext(getStringFromWasm0(arg1, arg2), getObject(arg3));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_target_cc69dde6c2d9ec90 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).target;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_cancelBubble_f67c419013823f11 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).cancelBubble;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_preventDefault_9866c9fd51eecfb6 = function() { return logError(function (arg0) {
        getObject(arg0).preventDefault();
    }, arguments) };
    imports.wbg.__wbg_stopPropagation_ae76be6b0f664ee8 = function() { return logError(function (arg0) {
        getObject(arg0).stopPropagation();
    }, arguments) };
    imports.wbg.__wbg_addEventListener_52721772cc0a7f30 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).addEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3));
    }, arguments) };
    imports.wbg.__wbg_addEventListener_09e11fbf8b4b719b = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).addEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3), getObject(arg4));
    }, arguments) };
    imports.wbg.__wbg_removeEventListener_f2adc9b2b318de99 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).removeEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3));
    }, arguments) };
    imports.wbg.__wbg_charCode_94907480ee48e8d5 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).charCode;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_keyCode_490ed69472addfdc = function() { return logError(function (arg0) {
        var ret = getObject(arg0).keyCode;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_altKey_3dcb50d5afbc5036 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).altKey;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_ctrlKey_fb62ba10b63b34a4 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).ctrlKey;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_shiftKey_bd2875540e5db840 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).shiftKey;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_metaKey_94ca09e07f21f240 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).metaKey;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_key_10dcaa4bb6d5449f = function() { return logError(function (arg0, arg1) {
        var ret = getObject(arg1).key;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    }, arguments) };
    imports.wbg.__wbg_code_1c2fd251f38b398a = function() { return logError(function (arg0, arg1) {
        var ret = getObject(arg1).code;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    }, arguments) };
    imports.wbg.__wbg_getModifierState_a3918fd0a6fed13b = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).getModifierState(getStringFromWasm0(arg1, arg2));
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_drawArraysInstancedANGLE_947637aa80c7e05d = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).drawArraysInstancedANGLE(arg1 >>> 0, arg2, arg3, arg4);
    }, arguments) };
    imports.wbg.__wbg_drawElementsInstancedANGLE_7cb606c4e0d0b01b = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).drawElementsInstancedANGLE(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
    }, arguments) };
    imports.wbg.__wbg_vertexAttribDivisorANGLE_844b4599bcb01375 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).vertexAttribDivisorANGLE(arg1 >>> 0, arg2 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_matches_76fae292b8cd60a6 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).matches;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_addListener_c86b1137b8521593 = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).addListener(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_removeListener_c880bdb0d8286afd = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).removeListener(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_instanceof_Blob_da9f4ebaadaae9ed = function() { return logError(function (arg0) {
        var ret = getObject(arg0) instanceof Blob;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_x_fbafdccc547c9d4b = function() { return logError(function (arg0) {
        var ret = getObject(arg0).x;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_y_6f4ed3bbef2ef02d = function() { return logError(function (arg0) {
        var ret = getObject(arg0).y;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_appendChild_d318db34c4559916 = function() { return handleError(function (arg0, arg1) {
        var ret = getObject(arg0).appendChild(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_body_78ae4fd43b446013 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).body;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_fullscreenElement_9c65b876dfef97a7 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).fullscreenElement;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createElement_86c152812a141a62 = function() { return handleError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).createElement(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_exitFullscreen_397b9dc3de048615 = function() { return logError(function (arg0) {
        getObject(arg0).exitFullscreen();
    }, arguments) };
    imports.wbg.__wbg_querySelector_b92a6c73bcfe671b = function() { return handleError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).querySelector(getStringFromWasm0(arg1, arg2));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getBoundingClientRect_2fba0402ea2a6ec4 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).getBoundingClientRect();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_requestFullscreen_03c1aaf2b58db6cb = function() { return handleError(function (arg0) {
        getObject(arg0).requestFullscreen();
    }, arguments) };
    imports.wbg.__wbg_setAttribute_1b533bf07966de55 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_setPointerCapture_bd3244d9f008798f = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).setPointerCapture(arg1);
    }, arguments) };
    imports.wbg.__wbg_bufferData_6be9c7fd06bae799 = function() { return logError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_bufferData_6beb22ecb30c1316 = function() { return logError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).bufferData(arg1 >>> 0, getObject(arg2), arg3 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_bufferSubData_179b18fb1177c92c = function() { return logError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).bufferSubData(arg1 >>> 0, arg2, getObject(arg3));
    }, arguments) };
    imports.wbg.__wbg_compressedTexSubImage2D_1a7c534d015590be = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
        getObject(arg0).compressedTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, getObject(arg8));
    }, arguments) };
    imports.wbg.__wbg_readPixels_65bbd6343831954a = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
        getObject(arg0).readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, arg7 === 0 ? undefined : getArrayU8FromWasm0(arg7, arg8));
    }, arguments) };
    imports.wbg.__wbg_texSubImage2D_92fbae5178901892 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
        getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
    }, arguments) };
    imports.wbg.__wbg_activeTexture_b34aca0c2110966c = function() { return logError(function (arg0, arg1) {
        getObject(arg0).activeTexture(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_attachShader_eaa824fd5b37a770 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).attachShader(getObject(arg1), getObject(arg2));
    }, arguments) };
    imports.wbg.__wbg_bindBuffer_2ca7e1c18819ecb2 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).bindBuffer(arg1 >>> 0, getObject(arg2));
    }, arguments) };
    imports.wbg.__wbg_bindFramebuffer_c9f468afa9d42a5f = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).bindFramebuffer(arg1 >>> 0, getObject(arg2));
    }, arguments) };
    imports.wbg.__wbg_bindRenderbuffer_7b2b49f71f3eeef9 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).bindRenderbuffer(arg1 >>> 0, getObject(arg2));
    }, arguments) };
    imports.wbg.__wbg_bindTexture_edd827f3dba6038e = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).bindTexture(arg1 >>> 0, getObject(arg2));
    }, arguments) };
    imports.wbg.__wbg_blendColor_2a9e7053c24106a2 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).blendColor(arg1, arg2, arg3, arg4);
    }, arguments) };
    imports.wbg.__wbg_blendEquation_a0d81c0d03c9d140 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).blendEquation(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_blendEquationSeparate_a6bd16f7ed531fb5 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_blendFunc_d5ab9f0ff5a40a48 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).blendFunc(arg1 >>> 0, arg2 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_blendFuncSeparate_815126bb13307b97 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_colorMask_035708eb94fe9af8 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).colorMask(arg1 !== 0, arg2 !== 0, arg3 !== 0, arg4 !== 0);
    }, arguments) };
    imports.wbg.__wbg_compileShader_8fb70a472f32552c = function() { return logError(function (arg0, arg1) {
        getObject(arg0).compileShader(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_copyTexSubImage2D_c9ff20fd4ccbc5f5 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
        getObject(arg0).copyTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8);
    }, arguments) };
    imports.wbg.__wbg_createBuffer_4802e2f0e1b1acdf = function() { return logError(function (arg0) {
        var ret = getObject(arg0).createBuffer();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createFramebuffer_0157699cdc720b46 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).createFramebuffer();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createProgram_b1d94f4c7554d3a1 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).createProgram();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createRenderbuffer_f10d2abe2f2c5aa4 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).createRenderbuffer();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createShader_da09e167692f0dc7 = function() { return logError(function (arg0, arg1) {
        var ret = getObject(arg0).createShader(arg1 >>> 0);
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createTexture_bafc7c08393ae59d = function() { return logError(function (arg0) {
        var ret = getObject(arg0).createTexture();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_cullFace_567e744e6243934e = function() { return logError(function (arg0, arg1) {
        getObject(arg0).cullFace(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_deleteBuffer_9c31f3452ba32db1 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).deleteBuffer(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_deleteFramebuffer_0f43513bd6c6d986 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).deleteFramebuffer(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_deleteProgram_a2c849932f79e7af = function() { return logError(function (arg0, arg1) {
        getObject(arg0).deleteProgram(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_deleteRenderbuffer_c623daba22fb2331 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).deleteRenderbuffer(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_deleteShader_a97b67b619baa0f0 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).deleteShader(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_deleteTexture_82d755a5ac828346 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).deleteTexture(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_depthFunc_2ffde5a067fe29a4 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).depthFunc(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_depthMask_0a99eff2e9451e0d = function() { return logError(function (arg0, arg1) {
        getObject(arg0).depthMask(arg1 !== 0);
    }, arguments) };
    imports.wbg.__wbg_depthRange_a56db343def3aeaf = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).depthRange(arg1, arg2);
    }, arguments) };
    imports.wbg.__wbg_disable_b07faddb7d04349f = function() { return logError(function (arg0, arg1) {
        getObject(arg0).disable(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_disableVertexAttribArray_c9fdabd5f12b0539 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).disableVertexAttribArray(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_drawArrays_deb0ae940b2e372b = function() { return logError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).drawArrays(arg1 >>> 0, arg2, arg3);
    }, arguments) };
    imports.wbg.__wbg_drawElements_8e8af4b6757fedce = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).drawElements(arg1 >>> 0, arg2, arg3 >>> 0, arg4);
    }, arguments) };
    imports.wbg.__wbg_enable_d3d210aeb08eff52 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).enable(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_enableVertexAttribArray_d539e547495bea44 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).enableVertexAttribArray(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_framebufferRenderbuffer_207dd7511415a5f8 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).framebufferRenderbuffer(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, getObject(arg4));
    }, arguments) };
    imports.wbg.__wbg_framebufferTexture2D_923c6fc6645661bc = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).framebufferTexture2D(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, getObject(arg4), arg5);
    }, arguments) };
    imports.wbg.__wbg_frontFace_1ae0f9f988bd176a = function() { return logError(function (arg0, arg1) {
        getObject(arg0).frontFace(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_getActiveUniform_b1f4a6da3779af76 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).getActiveUniform(getObject(arg1), arg2 >>> 0);
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getParameter_6412bd2d0602696d = function() { return handleError(function (arg0, arg1) {
        var ret = getObject(arg0).getParameter(arg1 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getProgramInfoLog_b60e82d52c200cbd = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg1).getProgramInfoLog(getObject(arg2));
        var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    }, arguments) };
    imports.wbg.__wbg_getProgramParameter_229c193895936bbe = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).getProgramParameter(getObject(arg1), arg2 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getShaderInfoLog_ba51160c01b98360 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg1).getShaderInfoLog(getObject(arg2));
        var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    }, arguments) };
    imports.wbg.__wbg_getShaderParameter_dadc55c10928575d = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).getShaderParameter(getObject(arg1), arg2 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getUniformLocation_c3b3570b4632cc5c = function() { return logError(function (arg0, arg1, arg2, arg3) {
        var ret = getObject(arg0).getUniformLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_linkProgram_7080c84b0233cea2 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).linkProgram(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_pixelStorei_3cd96723ae22a5c6 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).pixelStorei(arg1 >>> 0, arg2);
    }, arguments) };
    imports.wbg.__wbg_polygonOffset_be19e887f6545ec7 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).polygonOffset(arg1, arg2);
    }, arguments) };
    imports.wbg.__wbg_renderbufferStorage_25ffeed5caa0cc7c = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).renderbufferStorage(arg1 >>> 0, arg2 >>> 0, arg3, arg4);
    }, arguments) };
    imports.wbg.__wbg_scissor_35fe98c7da06091c = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).scissor(arg1, arg2, arg3, arg4);
    }, arguments) };
    imports.wbg.__wbg_shaderSource_67b991301db003d0 = function() { return logError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).shaderSource(getObject(arg1), getStringFromWasm0(arg2, arg3));
    }, arguments) };
    imports.wbg.__wbg_stencilFuncSeparate_b4ca3f41b30843d7 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).stencilFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3, arg4 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_stencilMask_0ec7662710f598d4 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).stencilMask(arg1 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_stencilMaskSeparate_f5dcff908c81a58f = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).stencilMaskSeparate(arg1 >>> 0, arg2 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_stencilOpSeparate_6aa1b90c17ae00b6 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).stencilOpSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_texParameteri_bd724f6a5ad0cbbc = function() { return logError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
    }, arguments) };
    imports.wbg.__wbg_uniform1i_0811c29c0eebe191 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).uniform1i(getObject(arg1), arg2);
    }, arguments) };
    imports.wbg.__wbg_uniform4f_c9cd7c0b5febd8e2 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).uniform4f(getObject(arg1), arg2, arg3, arg4, arg5);
    }, arguments) };
    imports.wbg.__wbg_useProgram_b72b0bfcbc720fa9 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).useProgram(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_vertexAttribPointer_b5cb524c6fe9eec8 = function() { return logError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
        getObject(arg0).vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
    }, arguments) };
    imports.wbg.__wbg_viewport_89af3aceb7036a2c = function() { return logError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).viewport(arg1, arg2, arg3, arg4);
    }, arguments) };
    imports.wbg.__wbg_style_c88e323890d3a091 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).style;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_error_a8f9d6c85b89d485 = function() { return logError(function (arg0, arg1) {
        console.error(getObject(arg0), getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_setProperty_1460c660bc329763 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).setProperty(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_code_18a1e10cf380ddc6 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).code;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_readyState_d14de08ffb5783f5 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).readyState;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_setonopen_33b75427f7db7ce1 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).onopen = getObject(arg1);
    }, arguments) };
    imports.wbg.__wbg_setonerror_cb55f0521ac0da3a = function() { return logError(function (arg0, arg1) {
        getObject(arg0).onerror = getObject(arg1);
    }, arguments) };
    imports.wbg.__wbg_setonclose_7094f96283d130e0 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).onclose = getObject(arg1);
    }, arguments) };
    imports.wbg.__wbg_setonmessage_ca5f75e4a84134ef = function() { return logError(function (arg0, arg1) {
        getObject(arg0).onmessage = getObject(arg1);
    }, arguments) };
    imports.wbg.__wbg_setbinaryType_c9a67ad8bb4125af = function() { return logError(function (arg0, arg1) {
        getObject(arg0).binaryType = takeObject(arg1);
    }, arguments) };
    imports.wbg.__wbg_new_982fe22cd93d67f7 = function() { return handleError(function (arg0, arg1) {
        var ret = new WebSocket(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_newwithstrsequence_15aec70da899280b = function() { return handleError(function (arg0, arg1, arg2) {
        var ret = new WebSocket(getStringFromWasm0(arg0, arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_close_f2a10c1de90df5f0 = function() { return handleError(function (arg0) {
        getObject(arg0).close();
    }, arguments) };
    imports.wbg.__wbg_send_503c2e7652e95bf5 = function() { return handleError(function (arg0, arg1, arg2) {
        getObject(arg0).send(getStringFromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_send_73ab09dbae2da3c3 = function() { return handleError(function (arg0, arg1, arg2) {
        getObject(arg0).send(getArrayU8FromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_drawBuffersWEBGL_e0a517bf12933c96 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).drawBuffersWEBGL(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_deltaX_8cfc6cd15e97d97c = function() { return logError(function (arg0) {
        var ret = getObject(arg0).deltaX;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_deltaY_080604c20160c0e8 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).deltaY;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_deltaMode_c5ec1ee518ea0a08 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).deltaMode;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_clientX_97ff0f5c7b19e687 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).clientX;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_clientY_cacd4a7e44b9719b = function() { return logError(function (arg0) {
        var ret = getObject(arg0).clientY;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_offsetX_5da3ebf8a8cda8a4 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).offsetX;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_offsetY_b0edbc16723a55cb = function() { return logError(function (arg0) {
        var ret = getObject(arg0).offsetY;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_ctrlKey_9761d22fa42f09c0 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).ctrlKey;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_shiftKey_78ee0fc1aa572c2e = function() { return logError(function (arg0) {
        var ret = getObject(arg0).shiftKey;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_altKey_8936038d973c56db = function() { return logError(function (arg0) {
        var ret = getObject(arg0).altKey;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_metaKey_e6b9e0aa35aa2974 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).metaKey;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_button_a02c0467d38e8338 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).button;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_buttons_9d7b6c334f0b37de = function() { return logError(function (arg0) {
        var ret = getObject(arg0).buttons;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_movementX_06830b9b9e6b695e = function() { return logError(function (arg0) {
        var ret = getObject(arg0).movementX;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_movementY_3e85d197fec41957 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).movementY;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_bindVertexArrayOES_7104c80fb0d31cb0 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).bindVertexArrayOES(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_createVertexArrayOES_ced4f8a084b789b7 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).createVertexArrayOES();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_deleteVertexArrayOES_82951d0fd3b32685 = function() { return logError(function (arg0, arg1) {
        getObject(arg0).deleteVertexArrayOES(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_pointerId_9302f0e125f0b48e = function() { return logError(function (arg0) {
        var ret = getObject(arg0).pointerId;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_instanceof_Response_e1b11afbefa5b563 = function() { return logError(function (arg0) {
        var ret = getObject(arg0) instanceof Response;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_arrayBuffer_b8937ed04beb0d36 = function() { return handleError(function (arg0) {
        var ret = getObject(arg0).arrayBuffer();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_wasClean_f06e0966f0a58bfa = function() { return logError(function (arg0) {
        var ret = getObject(arg0).wasClean;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_code_c8c420857439c0b4 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).code;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_reason_a10c58463722f72e = function() { return logError(function (arg0, arg1) {
        var ret = getObject(arg1).reason;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    }, arguments) };
    imports.wbg.__wbg_matches_563087e3acb03caf = function() { return logError(function (arg0) {
        var ret = getObject(arg0).matches;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_now_559193109055ebad = function() { return logError(function (arg0) {
        var ret = getObject(arg0).now();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_new_949bbc1147195c4e = function() { return logError(function () {
        var ret = new Array();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_get_67189fe0b323d288 = function() { return logError(function (arg0, arg1) {
        var ret = getObject(arg0)[arg1 >>> 0];
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_length_42e02f5a04d67464 = function() { return logError(function (arg0) {
        var ret = getObject(arg0).length;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_of_0df8f35f9ca22da0 = function() { return logError(function (arg0) {
        var ret = Array.of(getObject(arg0));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_push_284486ca27c6aa8b = function() { return logError(function (arg0, arg1) {
        var ret = getObject(arg0).push(getObject(arg1));
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_instanceof_ArrayBuffer_764b6d4119231cb3 = function() { return logError(function (arg0) {
        var ret = getObject(arg0) instanceof ArrayBuffer;
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_newnoargs_be86524d73f67598 = function() { return logError(function (arg0, arg1) {
        var ret = new Function(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_call_888d259a5fefc347 = function() { return handleError(function (arg0, arg1) {
        var ret = getObject(arg0).call(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_is_0f5efc7977a2c50b = function() { return logError(function (arg0, arg1) {
        var ret = Object.is(getObject(arg0), getObject(arg1));
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_new_0b83d3df67ecb33e = function() { return logError(function () {
        var ret = new Object();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_resolve_d23068002f584f22 = function() { return logError(function (arg0) {
        var ret = Promise.resolve(getObject(arg0));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_then_2fcac196782070cc = function() { return logError(function (arg0, arg1) {
        var ret = getObject(arg0).then(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_then_8c2d62e8ae5978f7 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_globalThis_3f735a5746d41fbd = function() { return handleError(function () {
        var ret = globalThis.globalThis;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_self_c6fbdfc2918d5e58 = function() { return handleError(function () {
        var ret = self.self;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_window_baec038b5ab35c54 = function() { return handleError(function () {
        var ret = window.window;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_global_1bc0b39582740e95 = function() { return handleError(function () {
        var ret = global.global;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_newwithbyteoffsetandlength_12c9ea92e6bd82ab = function() { return logError(function (arg0, arg1, arg2) {
        var ret = new Int8Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_newwithbyteoffsetandlength_cb1946ef06ced7aa = function() { return logError(function (arg0, arg1, arg2) {
        var ret = new Int16Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_newwithbyteoffsetandlength_c6cf704931530b90 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = new Int32Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_new_a7ce447f15ff496f = function() { return logError(function (arg0) {
        var ret = new Uint8Array(getObject(arg0));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_newwithlength_929232475839a482 = function() { return logError(function (arg0) {
        var ret = new Uint8Array(arg0 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_newwithbyteoffsetandlength_4b9b8c4e3f5adbff = function() { return logError(function (arg0, arg1, arg2) {
        var ret = new Uint8Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_subarray_8b658422a224f479 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = getObject(arg0).subarray(arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_length_1eb8fc608a0d4cdb = function() { return logError(function (arg0) {
        var ret = getObject(arg0).length;
        _assertNum(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_set_969ad0a60e51d320 = function() { return logError(function (arg0, arg1, arg2) {
        getObject(arg0).set(getObject(arg1), arg2 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_newwithbyteoffsetandlength_63181699967b1b75 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = new Uint16Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_newwithbyteoffsetandlength_4bc6fc57887d7f72 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = new Uint32Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_newwithbyteoffsetandlength_8bd669b4092b7244 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = new Float32Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_get_4d0f21c2f823742e = function() { return handleError(function (arg0, arg1) {
        var ret = Reflect.get(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_set_82a4e8a85e31ac42 = function() { return handleError(function (arg0, arg1, arg2) {
        var ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
        _assertBoolean(ret);
        return ret;
    }, arguments) };
    imports.wbg.__wbindgen_is_undefined = function(arg0) {
        var ret = getObject(arg0) === undefined;
        _assertBoolean(ret);
        return ret;
    };
    imports.wbg.__wbg_buffer_397eaa4d72ee94dd = function() { return logError(function (arg0) {
        var ret = getObject(arg0).buffer;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
        var ret = debugString(getObject(arg1));
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    };
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbindgen_memory = function() {
        var ret = wasm.memory;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper1921 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 146, __wbg_adapter_30);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbindgen_closure_wrapper6003 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 504, __wbg_adapter_33);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbindgen_closure_wrapper72844 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 3678, __wbg_adapter_36);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbindgen_closure_wrapper72846 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 3672, __wbg_adapter_39);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbindgen_closure_wrapper72848 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 3682, __wbg_adapter_42);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbindgen_closure_wrapper72850 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 3680, __wbg_adapter_45);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbindgen_closure_wrapper72852 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 3686, __wbg_adapter_48);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbindgen_closure_wrapper72854 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 3684, __wbg_adapter_51);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbindgen_closure_wrapper72856 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 3676, __wbg_adapter_54);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbindgen_closure_wrapper72858 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 3674, __wbg_adapter_57);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbindgen_closure_wrapper106858 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 6541, __wbg_adapter_60);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbindgen_closure_wrapper113965 = function() { return logError(function (arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 6752, __wbg_adapter_63);
        return addHeapObject(ret);
    }, arguments) };

    if (typeof input === 'string' || (typeof Request === 'function' && input instanceof Request) || (typeof URL === 'function' && input instanceof URL)) {
        input = fetch(input);
    }



    const { instance, module } = await load(await input, imports);

    wasm = instance.exports;
    init.__wbindgen_wasm_module = module;

    return wasm;
}

export default init;

