
let wasm;

const heap = new Array(32).fill(undefined);

heap.push(undefined, null, true, false);

function getObject(idx) { return heap[idx]; }

let heap_next = heap.length;

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

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    if (typeof(heap_next) !== 'number') throw new Error('corrupt heap');

    heap[idx] = obj;
    return idx;
}

function isLikeNone(x) {
    return x === undefined || x === null;
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

let cachegetInt32Memory0 = null;
function getInt32Memory0() {
    if (cachegetInt32Memory0 === null || cachegetInt32Memory0.buffer !== wasm.memory.buffer) {
        cachegetInt32Memory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachegetInt32Memory0;
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

function logError(f) {
    return function () {
        try {
            return f.apply(this, arguments);

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
    };
}
function __wbg_adapter_30(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h50dacd880a08fc34(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_33(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h94327245d18eb6e3(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_36(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h582f960fb1ae4fe2(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_39(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h4fba39b766b3dcbf(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_42(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hd0b036b8834b51f3(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_45(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h72d65b40ebf4ce0b(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_48(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h40cc7a062878c998(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_51(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h7bb232ef81e98684(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_54(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__haa729f63c131d3f4(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_57(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hbd2dba94ef61fa3a(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_60(arg0, arg1) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hfcb5b9d82f37b3ce(arg0, arg1);
}

function __wbg_adapter_63(arg0, arg1, arg2) {
    _assertNum(arg0);
    _assertNum(arg1);
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h601153b04f1d6774(arg0, arg1, addHeapObject(arg2));
}

/**
*/
export function run() {
    wasm.run();
}

function handleError(f) {
    return function () {
        try {
            return f.apply(this, arguments);

        } catch (e) {
            wasm.__wbindgen_exn_store(addHeapObject(e));
        }
    };
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
        input = import.meta.url.replace(/\.js$/, '_bg.wasm');
    }
    const imports = {};
    imports.wbg = {};
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
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        var ret = getStringFromWasm0(arg0, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_number_new = function(arg0) {
        var ret = arg0;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_object_clone_ref = function(arg0) {
        var ret = getObject(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_mark_55c5ecfe509deb6a = logError(function(arg0, arg1) {
        performance.mark(getStringFromWasm0(arg0, arg1));
    });
    imports.wbg.__wbg_measure_608563106d70d48a = logError(function(arg0, arg1, arg2, arg3) {
        performance.measure(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3));
    });
    imports.wbg.__wbg_log_b7dcb3facc73166d = logError(function(arg0, arg1) {
        console.log(getStringFromWasm0(arg0, arg1));
    });
    imports.wbg.__wbg_log_96295d68ab8338df = logError(function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
        console.log(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3), getStringFromWasm0(arg4, arg5), getStringFromWasm0(arg6, arg7));
    });
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbg_error_09919627ac0992f5 = logError(function(arg0, arg1) {
        try {
            console.error(getStringFromWasm0(arg0, arg1));
        } finally {
            wasm.__wbindgen_free(arg0, arg1);
        }
    });
    imports.wbg.__wbg_new_693216e109162396 = logError(function() {
        var ret = new Error();
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_stack_0ddaca5d1abfb52f = logError(function(arg0, arg1) {
        var ret = getObject(arg1).stack;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    });
    imports.wbg.__wbindgen_is_object = function(arg0) {
        const val = getObject(arg0);
        var ret = typeof(val) === 'object' && val !== null;
        _assertBoolean(ret);
        return ret;
    };
    imports.wbg.__wbg_msCrypto_a2cdb043d2bfe57f = logError(function(arg0) {
        var ret = getObject(arg0).msCrypto;
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_crypto_98fc271021c7d2ad = logError(function(arg0) {
        var ret = getObject(arg0).crypto;
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_getRandomValues_98117e9a7e993920 = handleError(function(arg0, arg1) {
        getObject(arg0).getRandomValues(getObject(arg1));
    });
    imports.wbg.__wbg_modulerequire_3440a4bcf44437db = handleError(function(arg0, arg1) {
        var ret = module.require(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_randomFillSync_64cc7d048f228ca8 = handleError(function(arg0, arg1, arg2) {
        getObject(arg0).randomFillSync(getArrayU8FromWasm0(arg1, arg2));
    });
    imports.wbg.__wbg_process_2f24d6544ea7b200 = logError(function(arg0) {
        var ret = getObject(arg0).process;
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_versions_6164651e75405d4a = logError(function(arg0) {
        var ret = getObject(arg0).versions;
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_node_4b517d861cbcb3bc = logError(function(arg0) {
        var ret = getObject(arg0).node;
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_instanceof_WebGl2RenderingContext_9818b789249374d3 = logError(function(arg0) {
        var ret = getObject(arg0) instanceof WebGL2RenderingContext;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_bindBufferRange_ec629985058604ae = logError(function(arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).bindBufferRange(arg1 >>> 0, arg2 >>> 0, getObject(arg3), arg4, arg5);
    });
    imports.wbg.__wbg_bindVertexArray_569f8b5466293fb0 = logError(function(arg0, arg1) {
        getObject(arg0).bindVertexArray(getObject(arg1));
    });
    imports.wbg.__wbg_bufferData_e6e272d30638e00b = logError(function(arg0, arg1, arg2, arg3) {
        getObject(arg0).bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
    });
    imports.wbg.__wbg_bufferData_8c572f7db0e55bdd = logError(function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).bufferData(arg1 >>> 0, getArrayU8FromWasm0(arg2, arg3), arg4 >>> 0);
    });
    imports.wbg.__wbg_bufferSubData_ff3883409f54dba5 = logError(function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
        getObject(arg0).bufferSubData(arg1 >>> 0, arg2, getArrayU8FromWasm0(arg3, arg4), arg5 >>> 0, arg6 >>> 0);
    });
    imports.wbg.__wbg_clearBufferfv_276f9cc79778aa2c = logError(function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).clearBufferfv(arg1 >>> 0, arg2, getArrayF32FromWasm0(arg3, arg4));
    });
    imports.wbg.__wbg_clearBufferuiv_5dff08850986bfa0 = logError(function(arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).clearBufferuiv(arg1 >>> 0, arg2, getArrayU32FromWasm0(arg3, arg4), arg5 >>> 0);
    });
    imports.wbg.__wbg_copyBufferSubData_ca3ab9c121aefd28 = logError(function(arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).copyBufferSubData(arg1 >>> 0, arg2 >>> 0, arg3, arg4, arg5);
    });
    imports.wbg.__wbg_createVertexArray_1f35f6d163bbae13 = logError(function(arg0) {
        var ret = getObject(arg0).createVertexArray();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_drawBuffers_0b800e44adca1dbf = logError(function(arg0, arg1) {
        getObject(arg0).drawBuffers(getObject(arg1));
    });
    imports.wbg.__wbg_drawElementsInstanced_e43707248d907aea = logError(function(arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).drawElementsInstanced(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
    });
    imports.wbg.__wbg_getActiveUniformBlockName_e8982440bd4f4256 = logError(function(arg0, arg1, arg2, arg3) {
        var ret = getObject(arg1).getActiveUniformBlockName(getObject(arg2), arg3 >>> 0);
        var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    });
    imports.wbg.__wbg_getActiveUniformBlockParameter_c7d46dbdce304742 = handleError(function(arg0, arg1, arg2, arg3) {
        var ret = getObject(arg0).getActiveUniformBlockParameter(getObject(arg1), arg2 >>> 0, arg3 >>> 0);
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_getBufferSubData_d7fa65ac13abe442 = logError(function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).getBufferSubData(arg1 >>> 0, arg2, getArrayU8FromWasm0(arg3, arg4));
    });
    imports.wbg.__wbg_getUniformBlockIndex_13d69d04aaa79521 = logError(function(arg0, arg1, arg2, arg3) {
        var ret = getObject(arg0).getUniformBlockIndex(getObject(arg1), getStringFromWasm0(arg2, arg3));
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_readBuffer_dff50171de17536c = logError(function(arg0, arg1) {
        getObject(arg0).readBuffer(arg1 >>> 0);
    });
    imports.wbg.__wbg_readPixels_f03368a55b5df242 = handleError(function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
        getObject(arg0).readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, arg7);
    });
    imports.wbg.__wbg_texImage2D_79c0e000ef5e1b0a = handleError(function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
        getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9 === 0 ? undefined : getArrayU8FromWasm0(arg9, arg10));
    });
    imports.wbg.__wbg_texImage2D_91e9f05dbc16878b = handleError(function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
        getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
    });
    imports.wbg.__wbg_texImage2D_a1f0626e2d955663 = handleError(function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
        getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9), arg10 >>> 0);
    });
    imports.wbg.__wbg_uniformBlockBinding_e11d75a8b7073f0b = logError(function(arg0, arg1, arg2, arg3) {
        getObject(arg0).uniformBlockBinding(getObject(arg1), arg2 >>> 0, arg3 >>> 0);
    });
    imports.wbg.__wbg_vertexAttribIPointer_982bac1182e02b2f = logError(function(arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).vertexAttribIPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
    });
    imports.wbg.__wbg_activeTexture_a756131b7b4547f3 = logError(function(arg0, arg1) {
        getObject(arg0).activeTexture(arg1 >>> 0);
    });
    imports.wbg.__wbg_attachShader_386953a8caf97e31 = logError(function(arg0, arg1, arg2) {
        getObject(arg0).attachShader(getObject(arg1), getObject(arg2));
    });
    imports.wbg.__wbg_bindBuffer_2cb370d7ee8c8faa = logError(function(arg0, arg1, arg2) {
        getObject(arg0).bindBuffer(arg1 >>> 0, getObject(arg2));
    });
    imports.wbg.__wbg_bindFramebuffer_4a37c2a7678c0994 = logError(function(arg0, arg1, arg2) {
        getObject(arg0).bindFramebuffer(arg1 >>> 0, getObject(arg2));
    });
    imports.wbg.__wbg_bindTexture_f3ab6393f75a763f = logError(function(arg0, arg1, arg2) {
        getObject(arg0).bindTexture(arg1 >>> 0, getObject(arg2));
    });
    imports.wbg.__wbg_blendEquation_76e42b66efb39144 = logError(function(arg0, arg1) {
        getObject(arg0).blendEquation(arg1 >>> 0);
    });
    imports.wbg.__wbg_blendFuncSeparate_3846af0a9de66b8d = logError(function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
    });
    imports.wbg.__wbg_checkFramebufferStatus_f742d2efafd5471f = logError(function(arg0, arg1) {
        var ret = getObject(arg0).checkFramebufferStatus(arg1 >>> 0);
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_clear_8e691dd4fbcdb78d = logError(function(arg0, arg1) {
        getObject(arg0).clear(arg1 >>> 0);
    });
    imports.wbg.__wbg_clearColor_c478bc8e70dd1fde = logError(function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).clearColor(arg1, arg2, arg3, arg4);
    });
    imports.wbg.__wbg_clearDepth_dcdd536856aabed0 = logError(function(arg0, arg1) {
        getObject(arg0).clearDepth(arg1);
    });
    imports.wbg.__wbg_compileShader_3c4bd5d4666a9951 = logError(function(arg0, arg1) {
        getObject(arg0).compileShader(getObject(arg1));
    });
    imports.wbg.__wbg_createBuffer_a9e0a9167dc2f2b4 = logError(function(arg0) {
        var ret = getObject(arg0).createBuffer();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_createFramebuffer_d01ac1b4f7c704e5 = logError(function(arg0) {
        var ret = getObject(arg0).createFramebuffer();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_createProgram_4823f8197c94860f = logError(function(arg0) {
        var ret = getObject(arg0).createProgram();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_createShader_9378e5028efeddcf = logError(function(arg0, arg1) {
        var ret = getObject(arg0).createShader(arg1 >>> 0);
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_createTexture_151a385cd028c893 = logError(function(arg0) {
        var ret = getObject(arg0).createTexture();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_cullFace_be96882240332455 = logError(function(arg0, arg1) {
        getObject(arg0).cullFace(arg1 >>> 0);
    });
    imports.wbg.__wbg_deleteBuffer_a983cfd5488ab211 = logError(function(arg0, arg1) {
        getObject(arg0).deleteBuffer(getObject(arg1));
    });
    imports.wbg.__wbg_deleteTexture_125ab82d8330e268 = logError(function(arg0, arg1) {
        getObject(arg0).deleteTexture(getObject(arg1));
    });
    imports.wbg.__wbg_depthFunc_1d638f5d5b4377b9 = logError(function(arg0, arg1) {
        getObject(arg0).depthFunc(arg1 >>> 0);
    });
    imports.wbg.__wbg_disable_5c31195749c90c83 = logError(function(arg0, arg1) {
        getObject(arg0).disable(arg1 >>> 0);
    });
    imports.wbg.__wbg_drawArrays_5793555840ecaa0b = logError(function(arg0, arg1, arg2, arg3) {
        getObject(arg0).drawArrays(arg1 >>> 0, arg2, arg3);
    });
    imports.wbg.__wbg_enable_f7d5513a12216046 = logError(function(arg0, arg1) {
        getObject(arg0).enable(arg1 >>> 0);
    });
    imports.wbg.__wbg_enableVertexAttribArray_3f2a29ade8fb65f9 = logError(function(arg0, arg1) {
        getObject(arg0).enableVertexAttribArray(arg1 >>> 0);
    });
    imports.wbg.__wbg_framebufferTexture2D_5b8575bda5aeceeb = logError(function(arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).framebufferTexture2D(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, getObject(arg4), arg5);
    });
    imports.wbg.__wbg_frontFace_70e23d09276ea052 = logError(function(arg0, arg1) {
        getObject(arg0).frontFace(arg1 >>> 0);
    });
    imports.wbg.__wbg_getActiveAttrib_aef25ffe66deb3ed = logError(function(arg0, arg1, arg2) {
        var ret = getObject(arg0).getActiveAttrib(getObject(arg1), arg2 >>> 0);
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_getActiveUniform_6c396bc6939f58db = logError(function(arg0, arg1, arg2) {
        var ret = getObject(arg0).getActiveUniform(getObject(arg1), arg2 >>> 0);
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_getAttribLocation_713a1d120f1e32ba = logError(function(arg0, arg1, arg2, arg3) {
        var ret = getObject(arg0).getAttribLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_getParameter_be1e4b3ba2c0c339 = handleError(function(arg0, arg1) {
        var ret = getObject(arg0).getParameter(arg1 >>> 0);
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_getProgramInfoLog_900722958284ce83 = logError(function(arg0, arg1, arg2) {
        var ret = getObject(arg1).getProgramInfoLog(getObject(arg2));
        var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    });
    imports.wbg.__wbg_getProgramParameter_7f66eafe63848c93 = logError(function(arg0, arg1, arg2) {
        var ret = getObject(arg0).getProgramParameter(getObject(arg1), arg2 >>> 0);
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_getShaderInfoLog_6e3d36e74e32aa2b = logError(function(arg0, arg1, arg2) {
        var ret = getObject(arg1).getShaderInfoLog(getObject(arg2));
        var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    });
    imports.wbg.__wbg_getShaderParameter_d3ad5fb12a1da258 = logError(function(arg0, arg1, arg2) {
        var ret = getObject(arg0).getShaderParameter(getObject(arg1), arg2 >>> 0);
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_getUniformLocation_02d298730d44dadc = logError(function(arg0, arg1, arg2, arg3) {
        var ret = getObject(arg0).getUniformLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_linkProgram_be955380b2064b69 = logError(function(arg0, arg1) {
        getObject(arg0).linkProgram(getObject(arg1));
    });
    imports.wbg.__wbg_scissor_967dc192f6260c23 = logError(function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).scissor(arg1, arg2, arg3, arg4);
    });
    imports.wbg.__wbg_shaderSource_0b51ed30c2234a07 = logError(function(arg0, arg1, arg2, arg3) {
        getObject(arg0).shaderSource(getObject(arg1), getStringFromWasm0(arg2, arg3));
    });
    imports.wbg.__wbg_texParameteri_6e7ba8c54bb639f2 = logError(function(arg0, arg1, arg2, arg3) {
        getObject(arg0).texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
    });
    imports.wbg.__wbg_uniform1i_2cb54693e4c3bace = logError(function(arg0, arg1, arg2) {
        getObject(arg0).uniform1i(getObject(arg1), arg2);
    });
    imports.wbg.__wbg_useProgram_6b54e2f64672af62 = logError(function(arg0, arg1) {
        getObject(arg0).useProgram(getObject(arg1));
    });
    imports.wbg.__wbg_vertexAttribPointer_12aeb3ec86d48d18 = logError(function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
        getObject(arg0).vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
    });
    imports.wbg.__wbg_viewport_ec826bf788ce964f = logError(function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).viewport(arg1, arg2, arg3, arg4);
    });
    imports.wbg.__wbg_instanceof_Window_49f532f06a9786ee = logError(function(arg0) {
        var ret = getObject(arg0) instanceof Window;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_document_c0366b39e4f4c89a = logError(function(arg0) {
        var ret = getObject(arg0).document;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_innerWidth_cea04a991524ea87 = handleError(function(arg0) {
        var ret = getObject(arg0).innerWidth;
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_innerHeight_83651dca462998d1 = handleError(function(arg0) {
        var ret = getObject(arg0).innerHeight;
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_devicePixelRatio_268c49438a600d53 = logError(function(arg0) {
        var ret = getObject(arg0).devicePixelRatio;
        return ret;
    });
    imports.wbg.__wbg_cancelAnimationFrame_60f9cf59ec1c0125 = handleError(function(arg0, arg1) {
        getObject(arg0).cancelAnimationFrame(arg1);
    });
    imports.wbg.__wbg_matchMedia_f9355258d56dc891 = handleError(function(arg0, arg1, arg2) {
        var ret = getObject(arg0).matchMedia(getStringFromWasm0(arg1, arg2));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_open_e75fc2c832f77be8 = handleError(function(arg0, arg1, arg2) {
        var ret = getObject(arg0).open(getStringFromWasm0(arg1, arg2));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_requestAnimationFrame_ef0e2294dc8b1088 = handleError(function(arg0, arg1) {
        var ret = getObject(arg0).requestAnimationFrame(getObject(arg1));
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_get_03d057a4fd2b7031 = logError(function(arg0, arg1, arg2) {
        var ret = getObject(arg0)[getStringFromWasm0(arg1, arg2)];
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_clearTimeout_cf42c747400433ba = logError(function(arg0, arg1) {
        getObject(arg0).clearTimeout(arg1);
    });
    imports.wbg.__wbg_fetch_b348373e5cdac8df = logError(function(arg0, arg1, arg2) {
        var ret = getObject(arg0).fetch(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_setTimeout_7df13099c62f73a7 = handleError(function(arg0, arg1, arg2) {
        var ret = getObject(arg0).setTimeout(getObject(arg1), arg2);
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_appendChild_7c45aeccd496f2a5 = handleError(function(arg0, arg1) {
        var ret = getObject(arg0).appendChild(getObject(arg1));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_instanceof_Blob_13ca80d39bf05976 = logError(function(arg0) {
        var ret = getObject(arg0) instanceof Blob;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_x_d61460e3c817f5b2 = logError(function(arg0) {
        var ret = getObject(arg0).x;
        return ret;
    });
    imports.wbg.__wbg_y_e4e5b87d074dc33d = logError(function(arg0) {
        var ret = getObject(arg0).y;
        return ret;
    });
    imports.wbg.__wbg_setProperty_46b9bd1b0fad730b = handleError(function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).setProperty(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    });
    imports.wbg.__wbg_readyState_8922ec81fb8a6bfe = logError(function(arg0) {
        var ret = getObject(arg0).readyState;
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_setonopen_c24d49cee44daad8 = logError(function(arg0, arg1) {
        getObject(arg0).onopen = getObject(arg1);
    });
    imports.wbg.__wbg_setonerror_64ee2a67b3f7eebf = logError(function(arg0, arg1) {
        getObject(arg0).onerror = getObject(arg1);
    });
    imports.wbg.__wbg_setonclose_2f8724fc70e9e861 = logError(function(arg0, arg1) {
        getObject(arg0).onclose = getObject(arg1);
    });
    imports.wbg.__wbg_setonmessage_fcc92da9f859cdc2 = logError(function(arg0, arg1) {
        getObject(arg0).onmessage = getObject(arg1);
    });
    imports.wbg.__wbg_setbinaryType_17d94084c919d157 = logError(function(arg0, arg1) {
        getObject(arg0).binaryType = takeObject(arg1);
    });
    imports.wbg.__wbg_new_df8fc59a35e30ae9 = handleError(function(arg0, arg1) {
        var ret = new WebSocket(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_newwithstrsequence_937bbc9e7f3f2bec = handleError(function(arg0, arg1, arg2) {
        var ret = new WebSocket(getStringFromWasm0(arg0, arg1), getObject(arg2));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_close_95e96a88da73491e = handleError(function(arg0) {
        getObject(arg0).close();
    });
    imports.wbg.__wbg_send_c6982a9ac7d4b83d = handleError(function(arg0, arg1, arg2) {
        getObject(arg0).send(getStringFromWasm0(arg1, arg2));
    });
    imports.wbg.__wbg_send_293e3fd21850ea7d = handleError(function(arg0, arg1, arg2) {
        getObject(arg0).send(getArrayU8FromWasm0(arg1, arg2));
    });
    imports.wbg.__wbg_clientX_3a14a1583294607f = logError(function(arg0) {
        var ret = getObject(arg0).clientX;
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_clientY_4b4a322b80551002 = logError(function(arg0) {
        var ret = getObject(arg0).clientY;
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_offsetX_4bd8c9fcb457cf0b = logError(function(arg0) {
        var ret = getObject(arg0).offsetX;
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_offsetY_0dde12490e8ebfba = logError(function(arg0) {
        var ret = getObject(arg0).offsetY;
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_ctrlKey_fadbf4d226c5a071 = logError(function(arg0) {
        var ret = getObject(arg0).ctrlKey;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_shiftKey_6df8deff50c0048c = logError(function(arg0) {
        var ret = getObject(arg0).shiftKey;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_altKey_470315032c1b4a35 = logError(function(arg0) {
        var ret = getObject(arg0).altKey;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_metaKey_42ae5f8d628a98d5 = logError(function(arg0) {
        var ret = getObject(arg0).metaKey;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_button_9e74bd912190b055 = logError(function(arg0) {
        var ret = getObject(arg0).button;
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_buttons_5d3db1e47542f585 = logError(function(arg0) {
        var ret = getObject(arg0).buttons;
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_code_9bdbf4180364e05d = logError(function(arg0) {
        var ret = getObject(arg0).code;
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_instanceof_Response_f52c65c389890639 = logError(function(arg0) {
        var ret = getObject(arg0) instanceof Response;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_arrayBuffer_0ba17dfaad804b6f = handleError(function(arg0) {
        var ret = getObject(arg0).arrayBuffer();
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_deltaX_5fac4f36a42e6ec9 = logError(function(arg0) {
        var ret = getObject(arg0).deltaX;
        return ret;
    });
    imports.wbg.__wbg_deltaY_2722120e563d3160 = logError(function(arg0) {
        var ret = getObject(arg0).deltaY;
        return ret;
    });
    imports.wbg.__wbg_deltaMode_3db3c9c4bedf191d = logError(function(arg0) {
        var ret = getObject(arg0).deltaMode;
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_wasClean_461f27545be1608c = logError(function(arg0) {
        var ret = getObject(arg0).wasClean;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_code_4a5510df7ab7d940 = logError(function(arg0) {
        var ret = getObject(arg0).code;
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_reason_ad2cc1a1e8e17595 = logError(function(arg0, arg1) {
        var ret = getObject(arg1).reason;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    });
    imports.wbg.__wbg_matches_c1680f96c1f19da4 = logError(function(arg0) {
        var ret = getObject(arg0).matches;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_now_7628760b7b640632 = logError(function(arg0) {
        var ret = getObject(arg0).now();
        return ret;
    });
    imports.wbg.__wbg_pointerId_602db5c989b38cc0 = logError(function(arg0) {
        var ret = getObject(arg0).pointerId;
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_type_69df81ce730cd07a = logError(function(arg0) {
        var ret = getObject(arg0).type;
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_name_99c5f2c3a3d268ab = logError(function(arg0, arg1) {
        var ret = getObject(arg1).name;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    });
    imports.wbg.__wbg_body_c8cb19d760637268 = logError(function(arg0) {
        var ret = getObject(arg0).body;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_fullscreenElement_40ed1ecabc8c860a = logError(function(arg0) {
        var ret = getObject(arg0).fullscreenElement;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_createElement_99351c8bf0efac6e = handleError(function(arg0, arg1, arg2) {
        var ret = getObject(arg0).createElement(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_exitFullscreen_5cd6f888225ba968 = logError(function(arg0) {
        getObject(arg0).exitFullscreen();
    });
    imports.wbg.__wbg_querySelector_f7730f338b4d3d21 = handleError(function(arg0, arg1, arg2) {
        var ret = getObject(arg0).querySelector(getStringFromWasm0(arg1, arg2));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_getBoundingClientRect_505844bd8eb35668 = logError(function(arg0) {
        var ret = getObject(arg0).getBoundingClientRect();
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_requestFullscreen_60b4644a038d0689 = handleError(function(arg0) {
        getObject(arg0).requestFullscreen();
    });
    imports.wbg.__wbg_setAttribute_e71b9086539f06a1 = handleError(function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    });
    imports.wbg.__wbg_setPointerCapture_54ee987062d42d03 = handleError(function(arg0, arg1) {
        getObject(arg0).setPointerCapture(arg1);
    });
    imports.wbg.__wbg_style_9b773f0fc441eddc = logError(function(arg0) {
        var ret = getObject(arg0).style;
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_error_d58d9958868010f6 = logError(function(arg0, arg1) {
        console.error(getObject(arg0), getObject(arg1));
    });
    imports.wbg.__wbg_addEventListener_6a37bc32387cb66d = handleError(function(arg0, arg1, arg2, arg3) {
        getObject(arg0).addEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3));
    });
    imports.wbg.__wbg_addEventListener_a422088e686210b5 = handleError(function(arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).addEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3), getObject(arg4));
    });
    imports.wbg.__wbg_removeEventListener_70dfb387da1982ac = handleError(function(arg0, arg1, arg2, arg3) {
        getObject(arg0).removeEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3));
    });
    imports.wbg.__wbg_data_5c896013c39c6e21 = logError(function(arg0) {
        var ret = getObject(arg0).data;
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_target_4bc4eb28204bcc44 = logError(function(arg0) {
        var ret = getObject(arg0).target;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_cancelBubble_62eb67fd286e013f = logError(function(arg0) {
        var ret = getObject(arg0).cancelBubble;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_preventDefault_9aab6c264e5df3ee = logError(function(arg0) {
        getObject(arg0).preventDefault();
    });
    imports.wbg.__wbg_stopPropagation_697200010cec9b7e = logError(function(arg0) {
        getObject(arg0).stopPropagation();
    });
    imports.wbg.__wbg_instanceof_HtmlCanvasElement_7bd3ee7838f11fc3 = logError(function(arg0) {
        var ret = getObject(arg0) instanceof HTMLCanvasElement;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_width_0efa4604d41c58c5 = logError(function(arg0) {
        var ret = getObject(arg0).width;
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_setwidth_1d0e975feecff3ef = logError(function(arg0, arg1) {
        getObject(arg0).width = arg1 >>> 0;
    });
    imports.wbg.__wbg_height_aa24e3fef658c4a8 = logError(function(arg0) {
        var ret = getObject(arg0).height;
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_setheight_7758ee3ff5c65474 = logError(function(arg0, arg1) {
        getObject(arg0).height = arg1 >>> 0;
    });
    imports.wbg.__wbg_getContext_93be69215ea9dbbf = handleError(function(arg0, arg1, arg2, arg3) {
        var ret = getObject(arg0).getContext(getStringFromWasm0(arg1, arg2), getObject(arg3));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_charCode_eb123e299efafe3f = logError(function(arg0) {
        var ret = getObject(arg0).charCode;
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_keyCode_47f9e9228bc483bf = logError(function(arg0) {
        var ret = getObject(arg0).keyCode;
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_altKey_8a59e1cf32636010 = logError(function(arg0) {
        var ret = getObject(arg0).altKey;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_ctrlKey_17377b46ca5a072d = logError(function(arg0) {
        var ret = getObject(arg0).ctrlKey;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_shiftKey_09be9a7e6cad7a99 = logError(function(arg0) {
        var ret = getObject(arg0).shiftKey;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_metaKey_a707288e6c45a0e0 = logError(function(arg0) {
        var ret = getObject(arg0).metaKey;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_key_d9b602f48baca7bc = logError(function(arg0, arg1) {
        var ret = getObject(arg1).key;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    });
    imports.wbg.__wbg_code_cbf76ad384ae1179 = logError(function(arg0, arg1) {
        var ret = getObject(arg1).code;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    });
    imports.wbg.__wbg_getModifierState_e62cfa723da709b4 = logError(function(arg0, arg1, arg2) {
        var ret = getObject(arg0).getModifierState(getStringFromWasm0(arg1, arg2));
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_matches_2f8453eb8e607f46 = logError(function(arg0) {
        var ret = getObject(arg0).matches;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_addListener_34d9bdd94b12c993 = handleError(function(arg0, arg1) {
        getObject(arg0).addListener(getObject(arg1));
    });
    imports.wbg.__wbg_removeListener_5571e3bc24e85d2c = handleError(function(arg0, arg1) {
        getObject(arg0).removeListener(getObject(arg1));
    });
    imports.wbg.__wbg_new_9dff83a08f5994f3 = logError(function() {
        var ret = new Array();
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_push_3ddd8187ff2ff82d = logError(function(arg0, arg1) {
        var ret = getObject(arg0).push(getObject(arg1));
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_instanceof_ArrayBuffer_3a0fa134e6809d57 = logError(function(arg0) {
        var ret = getObject(arg0) instanceof ArrayBuffer;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_newnoargs_7c6bd521992b4022 = logError(function(arg0, arg1) {
        var ret = new Function(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_call_951bd0c6d815d6f1 = handleError(function(arg0, arg1) {
        var ret = getObject(arg0).call(getObject(arg1));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_is_049b1aece40b5301 = logError(function(arg0, arg1) {
        var ret = Object.is(getObject(arg0), getObject(arg1));
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_new_ba07d0daa0e4677e = logError(function() {
        var ret = new Object();
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_resolve_6e61e640925a0db9 = logError(function(arg0) {
        var ret = Promise.resolve(getObject(arg0));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_then_dd3785597974798a = logError(function(arg0, arg1) {
        var ret = getObject(arg0).then(getObject(arg1));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_then_0f957e0f4c3e537a = logError(function(arg0, arg1, arg2) {
        var ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_globalThis_513fb247e8e4e6d2 = handleError(function() {
        var ret = globalThis.globalThis;
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_self_6baf3a3aa7b63415 = handleError(function() {
        var ret = self.self;
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_window_63fc4027b66c265b = handleError(function() {
        var ret = window.window;
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_global_b87245cd886d7113 = handleError(function() {
        var ret = global.global;
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_instanceof_Int32Array_49c362cd8a1d3dba = logError(function(arg0) {
        var ret = getObject(arg0) instanceof Int32Array;
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbg_getindex_65894fe7a532198d = logError(function(arg0, arg1) {
        var ret = getObject(arg0)[arg1 >>> 0];
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_new_c6c0228e6d22a2f9 = logError(function(arg0) {
        var ret = new Uint8Array(getObject(arg0));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_newwithlength_a429e08f8a8fe4b3 = logError(function(arg0) {
        var ret = new Uint8Array(arg0 >>> 0);
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_subarray_02e2fcfa6b285cb2 = logError(function(arg0, arg1, arg2) {
        var ret = getObject(arg0).subarray(arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_length_c645e7c02233b440 = logError(function(arg0) {
        var ret = getObject(arg0).length;
        _assertNum(ret);
        return ret;
    });
    imports.wbg.__wbg_set_b91afac9fd216d99 = logError(function(arg0, arg1, arg2) {
        getObject(arg0).set(getObject(arg1), arg2 >>> 0);
    });
    imports.wbg.__wbg_newwithbyteoffsetandlength_2016b902c412c87c = logError(function(arg0, arg1, arg2) {
        var ret = new Uint32Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_get_85e0a3b459845fe2 = handleError(function(arg0, arg1) {
        var ret = Reflect.get(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_set_9bdd413385146137 = handleError(function(arg0, arg1, arg2) {
        var ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
        _assertBoolean(ret);
        return ret;
    });
    imports.wbg.__wbindgen_is_undefined = function(arg0) {
        var ret = getObject(arg0) === undefined;
        _assertBoolean(ret);
        return ret;
    };
    imports.wbg.__wbg_buffer_3f12a1c608c6d04e = logError(function(arg0) {
        var ret = getObject(arg0).buffer;
        return addHeapObject(ret);
    });
    imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        var ret = typeof(obj) === 'number' ? obj : undefined;
        if (!isLikeNone(ret)) {
            _assertNum(ret);
        }
        getFloat64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? 0 : ret;
        getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
    };
    imports.wbg.__wbindgen_is_string = function(arg0) {
        var ret = typeof(getObject(arg0)) === 'string';
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
    imports.wbg.__wbindgen_boolean_get = function(arg0) {
        const v = getObject(arg0);
        var ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
        _assertNum(ret);
        return ret;
    };
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
    imports.wbg.__wbindgen_closure_wrapper1404 = logError(function(arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 267, __wbg_adapter_30);
        return addHeapObject(ret);
    });
    imports.wbg.__wbindgen_closure_wrapper6130 = logError(function(arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 475, __wbg_adapter_33);
        return addHeapObject(ret);
    });
    imports.wbg.__wbindgen_closure_wrapper21638 = logError(function(arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 882, __wbg_adapter_36);
        return addHeapObject(ret);
    });
    imports.wbg.__wbindgen_closure_wrapper21640 = logError(function(arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 872, __wbg_adapter_39);
        return addHeapObject(ret);
    });
    imports.wbg.__wbindgen_closure_wrapper21642 = logError(function(arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 878, __wbg_adapter_42);
        return addHeapObject(ret);
    });
    imports.wbg.__wbindgen_closure_wrapper21644 = logError(function(arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 874, __wbg_adapter_45);
        return addHeapObject(ret);
    });
    imports.wbg.__wbindgen_closure_wrapper21646 = logError(function(arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 868, __wbg_adapter_48);
        return addHeapObject(ret);
    });
    imports.wbg.__wbindgen_closure_wrapper21648 = logError(function(arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 870, __wbg_adapter_51);
        return addHeapObject(ret);
    });
    imports.wbg.__wbindgen_closure_wrapper21650 = logError(function(arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 876, __wbg_adapter_54);
        return addHeapObject(ret);
    });
    imports.wbg.__wbindgen_closure_wrapper21652 = logError(function(arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 880, __wbg_adapter_57);
        return addHeapObject(ret);
    });
    imports.wbg.__wbindgen_closure_wrapper100701 = logError(function(arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 9268, __wbg_adapter_60);
        return addHeapObject(ret);
    });
    imports.wbg.__wbindgen_closure_wrapper113569 = logError(function(arg0, arg1, arg2) {
        var ret = makeMutClosure(arg0, arg1, 10818, __wbg_adapter_63);
        return addHeapObject(ret);
    });

    if (typeof input === 'string' || (typeof Request === 'function' && input instanceof Request) || (typeof URL === 'function' && input instanceof URL)) {
        input = fetch(input);
    }

    const { instance, module } = await load(await input, imports);

    wasm = instance.exports;
    init.__wbindgen_wasm_module = module;

    return wasm;
}

export default init;
