// This code implements the `-sMODULARIZE` settings by taking the generated
// JS program code (INNER_JS_CODE) and wrapping it in a factory function.

// Single threaded MINIMAL_RUNTIME programs do not need access to
// document.currentScript, so a simple export declaration is enough.
var TreeSheets = (() => {
  // When MODULARIZE this JS may be executed later,
  // after document.currentScript is gone, so we save it.
  // In EXPORT_ES6 mode we can just use 'import.meta.url'.
  var _scriptName = globalThis.document?.currentScript?.src;
  return async function(moduleArg = {}) {
    var moduleRtn;

// include: shell.js
// include: minimum_runtime_check.js
(function() {
  // "30.0.0" -> 300000
  function humanReadableVersionToPacked(str) {
    str = str.split('-')[0]; // Remove any trailing part from e.g. "12.53.3-alpha"
    var vers = str.split('.').slice(0, 3);
    while(vers.length < 3) vers.push('00');
    vers = vers.map((n, i, arr) => n.padStart(2, '0'));
    return vers.join('');
  }
  // 300000 -> "30.0.0"
  var packedVersionToHumanReadable = n => [n / 10000 | 0, (n / 100 | 0) % 100, n % 100].join('.');

  var TARGET_NOT_SUPPORTED = 2147483647;

  // Note: We use a typeof check here instead of optional chaining using
  // globalThis because older browsers might not have globalThis defined.
  var currentNodeVersion = typeof process !== 'undefined' && process.versions?.node ? humanReadableVersionToPacked(process.versions.node) : TARGET_NOT_SUPPORTED;
  if (currentNodeVersion < TARGET_NOT_SUPPORTED) {
    throw new Error('not compiled for this environment (did you build to HTML and try to run it not on the web, or set ENVIRONMENT to something - like node - and run it someplace else - like on the web?)');
  }
  if (currentNodeVersion < 2147483647) {
    throw new Error(`This emscripten-generated code requires node v${ packedVersionToHumanReadable(2147483647) } (detected v${packedVersionToHumanReadable(currentNodeVersion)})`);
  }

  var userAgent = typeof navigator !== 'undefined' && navigator.userAgent;
  if (!userAgent) {
    return;
  }

  var currentSafariVersion = userAgent.includes("Safari/") && !userAgent.includes("Chrome/") && userAgent.match(/Version\/(\d+\.?\d*\.?\d*)/) ? humanReadableVersionToPacked(userAgent.match(/Version\/(\d+\.?\d*\.?\d*)/)[1]) : TARGET_NOT_SUPPORTED;
  if (currentSafariVersion < 150000) {
    throw new Error(`This emscripten-generated code requires Safari v${ packedVersionToHumanReadable(150000) } (detected v${currentSafariVersion})`);
  }

  var currentFirefoxVersion = userAgent.match(/Firefox\/(\d+(?:\.\d+)?)/) ? parseFloat(userAgent.match(/Firefox\/(\d+(?:\.\d+)?)/)[1]) : TARGET_NOT_SUPPORTED;
  if (currentFirefoxVersion < 79) {
    throw new Error(`This emscripten-generated code requires Firefox v79 (detected v${currentFirefoxVersion})`);
  }

  var currentChromeVersion = userAgent.match(/Chrome\/(\d+(?:\.\d+)?)/) ? parseFloat(userAgent.match(/Chrome\/(\d+(?:\.\d+)?)/)[1]) : TARGET_NOT_SUPPORTED;
  if (currentChromeVersion < 85) {
    throw new Error(`This emscripten-generated code requires Chrome v85 (detected v${currentChromeVersion})`);
  }
})();

// end include: minimum_runtime_check.js
// The Module object: Our interface to the outside world. We import
// and export values on it. There are various ways Module can be used:
// 1. Not defined. We create it here
// 2. A function parameter, function(moduleArg) => Promise<Module>
// 3. pre-run appended it, var Module = {}; ..generated code..
// 4. External script tag defines var Module.
// We need to check if Module already exists (e.g. case 3 above).
// Substitution will be replaced with actual code on later stage of the build,
// this way Closure Compiler will not mangle it (e.g. case 4. above).
// Note that if you want to run closure, and also to use Module
// after the generated code, you will need to define   var Module = {};
// before the code. Then that object will be used in the code, and you
// can continue to use Module afterwards as well.
var Module = moduleArg;

// Determine the runtime environment we are in. You can customize this by
// setting the ENVIRONMENT setting at compile time (see settings.js).

var ENVIRONMENT_IS_WEB = true;
var ENVIRONMENT_IS_WORKER = false;
var ENVIRONMENT_IS_NODE = false;
var ENVIRONMENT_IS_SHELL = false;

// --pre-jses are emitted after the Module integration code, so that they can
// refer to Module (if they choose; they can also define Module)


var arguments_ = [];
var thisProgram = './this.program';
var quit_ = (status, toThrow) => {
  throw toThrow;
};

// `/` should be present at the end if `scriptDirectory` is not empty
var scriptDirectory = '';
function locateFile(path) {
  if (Module['locateFile']) {
    return Module['locateFile'](path, scriptDirectory);
  }
  return scriptDirectory + path;
}

// Hooks that are implemented differently in different runtime environments.
var readAsync, readBinary;

if (ENVIRONMENT_IS_SHELL) {

} else

// Note that this includes Node.js workers when relevant (pthreads is enabled).
// Node.js workers are detected as a combination of ENVIRONMENT_IS_WORKER and
// ENVIRONMENT_IS_NODE.
if (ENVIRONMENT_IS_WEB || ENVIRONMENT_IS_WORKER) {
  try {
    scriptDirectory = new URL('.', _scriptName).href; // includes trailing slash
  } catch {
    // Must be a `blob:` or `data:` URL (e.g. `blob:http://site.com/etc/etc`), we cannot
    // infer anything from them.
  }

  if (!(globalThis.window || globalThis.WorkerGlobalScope)) throw new Error('not compiled for this environment (did you build to HTML and try to run it not on the web, or set ENVIRONMENT to something - like node - and run it someplace else - like on the web?)');

  {
// include: web_or_worker_shell_read.js
readAsync = async (url) => {
    assert(!isFileURI(url), "readAsync does not work with file:// URLs");
    var response = await fetch(url, { credentials: 'same-origin' });
    if (response.ok) {
      return response.arrayBuffer();
    }
    throw new Error(response.status + ' : ' + response.url);
  };
// end include: web_or_worker_shell_read.js
  }
} else
{
  throw new Error('environment detection error');
}

var out = console.log.bind(console);
var err = console.error.bind(console);

var IDBFS = 'IDBFS is no longer included by default; build with -lidbfs.js';
var PROXYFS = 'PROXYFS is no longer included by default; build with -lproxyfs.js';
var WORKERFS = 'WORKERFS is no longer included by default; build with -lworkerfs.js';
var FETCHFS = 'FETCHFS is no longer included by default; build with -lfetchfs.js';
var ICASEFS = 'ICASEFS is no longer included by default; build with -licasefs.js';
var JSFILEFS = 'JSFILEFS is no longer included by default; build with -ljsfilefs.js';
var OPFS = 'OPFS is no longer included by default; build with -lopfs.js';

var NODEFS = 'NODEFS is no longer included by default; build with -lnodefs.js';

// perform assertions in shell.js after we set up out() and err(), as otherwise
// if an assertion fails it cannot print the message

assert(!ENVIRONMENT_IS_WORKER, 'worker environment detected but not enabled at build time.  Add `worker` to `-sENVIRONMENT` to enable.');

assert(!ENVIRONMENT_IS_NODE, 'node environment detected but not enabled at build time.  Add `node` to `-sENVIRONMENT` to enable.');

assert(!ENVIRONMENT_IS_SHELL, 'shell environment detected but not enabled at build time.  Add `shell` to `-sENVIRONMENT` to enable.');

// end include: shell.js

// include: preamble.js
// === Preamble library stuff ===

// Documentation for the public APIs defined in this file must be updated in:
//    site/source/docs/api_reference/preamble.js.rst
// A prebuilt local version of the documentation is available at:
//    site/build/text/docs/api_reference/preamble.js.txt
// You can also build docs locally as HTML or other formats in site/
// An online HTML version (which may be of a different version of Emscripten)
//    is up at http://kripken.github.io/emscripten-site/docs/api_reference/preamble.js.html

var wasmBinary;

if (!globalThis.WebAssembly) {
  err('no native wasm support detected');
}

// Wasm globals

//========================================
// Runtime essentials
//========================================

// whether we are quitting the application. no code should run after this.
// set in exit() and abort()
var ABORT = false;

// set by exit() and abort().  Passed to 'onExit' handler.
// NOTE: This is also used as the process return code code in shell environments
// but only when noExitRuntime is false.
var EXITSTATUS;

// In STRICT mode, we only define assert() when ASSERTIONS is set.  i.e. we
// don't define it at all in release modes.  This matches the behaviour of
// MINIMAL_RUNTIME.
// TODO(sbc): Make this the default even without STRICT enabled.
/** @type {function(*, string=)} */
function assert(condition, text) {
  if (!condition) {
    abort('Assertion failed' + (text ? ': ' + text : ''));
  }
}

// We used to include malloc/free by default in the past. Show a helpful error in
// builds with assertions.
function _malloc() {
  abort('malloc() called but not included in the build - add `_malloc` to EXPORTED_FUNCTIONS');
}
function _free() {
  // Show a helpful error since we used to include free by default in the past.
  abort('free() called but not included in the build - add `_free` to EXPORTED_FUNCTIONS');
}

/**
 * Indicates whether filename is delivered via file protocol (as opposed to http/https)
 * @noinline
 */
var isFileURI = (filename) => filename.startsWith('file://');

// include: runtime_common.js
// include: runtime_stack_check.js
// Initializes the stack cookie. Called at the startup of main and at the startup of each thread in pthreads mode.
function writeStackCookie() {
  var max = _emscripten_stack_get_end();
  assert((max & 3) == 0);
  // If the stack ends at address zero we write our cookies 4 bytes into the
  // stack.  This prevents interference with SAFE_HEAP and ASAN which also
  // monitor writes to address zero.
  if (max == 0) {
    max += 4;
  }
  // The stack grow downwards towards _emscripten_stack_get_end.
  // We write cookies to the final two words in the stack and detect if they are
  // ever overwritten.
  HEAPU32[((max)>>2)] = 0x02135467;checkInt32(0x02135467);
  HEAPU32[(((max)+(4))>>2)] = 0x89BACDFE;checkInt32(0x89BACDFE);
  // Also test the global address 0 for integrity.
  HEAPU32[((0)>>2)] = 1668509029;checkInt32(1668509029);
}

function checkStackCookie() {
  if (ABORT) return;
  var max = _emscripten_stack_get_end();
  // See writeStackCookie().
  if (max == 0) {
    max += 4;
  }
  var cookie1 = HEAPU32[((max)>>2)];
  var cookie2 = HEAPU32[(((max)+(4))>>2)];
  if (cookie1 != 0x02135467 || cookie2 != 0x89BACDFE) {
    abort(`Stack overflow! Stack cookie has been overwritten at ${ptrToString(max)}, expected hex dwords 0x89BACDFE and 0x2135467, but received ${ptrToString(cookie2)} ${ptrToString(cookie1)}`);
  }
  // Also test the global address 0 for integrity.
  if (HEAPU32[((0)>>2)] != 0x63736d65 /* 'emsc' */) {
    abort('Runtime error: The application has corrupted its heap memory area (address zero)!');
  }
}
// end include: runtime_stack_check.js
// include: runtime_exceptions.js
// end include: runtime_exceptions.js
// include: runtime_debug.js
var runtimeDebug = true; // Switch to false at runtime to disable logging at the right times

// Used by XXXXX_DEBUG settings to output debug messages.
function dbg(...args) {
  if (!runtimeDebug && typeof runtimeDebug != 'undefined') return;
  // TODO(sbc): Make this configurable somehow.  Its not always convenient for
  // logging to show up as warnings.
  console.warn(...args);
}

// Endianness check
(() => {
  var h16 = new Int16Array(1);
  var h8 = new Int8Array(h16.buffer);
  h16[0] = 0x6373;
  if (h8[0] !== 0x73 || h8[1] !== 0x63) abort('Runtime error: expected the system to be little-endian! (Run with -sSUPPORT_BIG_ENDIAN to bypass)');
})();

function consumedModuleProp(prop) {
  if (!Object.getOwnPropertyDescriptor(Module, prop)) {
    Object.defineProperty(Module, prop, {
      configurable: true,
      set() {
        abort(`Attempt to set \`Module.${prop}\` after it has already been processed.  This can happen, for example, when code is injected via '--post-js' rather than '--pre-js'`);

      }
    });
  }
}

function makeInvalidEarlyAccess(name) {
  return () => assert(false, `call to '${name}' via reference taken before Wasm module initialization`);

}

function ignoredModuleProp(prop) {
  if (Object.getOwnPropertyDescriptor(Module, prop)) {
    abort(`\`Module.${prop}\` was supplied but \`${prop}\` not included in INCOMING_MODULE_JS_API`);
  }
}

// forcing the filesystem exports a few things by default
function isExportedByForceFilesystem(name) {
  return name === 'FS_createPath' ||
         name === 'FS_createDataFile' ||
         name === 'FS_createPreloadedFile' ||
         name === 'FS_preloadFile' ||
         name === 'FS_unlink' ||
         name === 'addRunDependency' ||
         // The old FS has some functionality that WasmFS lacks.
         name === 'FS_createLazyFile' ||
         name === 'FS_createDevice' ||
         name === 'removeRunDependency';
}

function missingLibrarySymbol(sym) {

  // Any symbol that is not included from the JS library is also (by definition)
  // not exported on the Module object.
  unexportedRuntimeSymbol(sym);
}

function unexportedRuntimeSymbol(sym) {
  if (!Object.getOwnPropertyDescriptor(Module, sym)) {
    Object.defineProperty(Module, sym, {
      configurable: true,
      get() {
        var msg = `'${sym}' was not exported. add it to EXPORTED_RUNTIME_METHODS (see the Emscripten FAQ)`;
        if (isExportedByForceFilesystem(sym)) {
          msg += '. Alternatively, forcing filesystem support (-sFORCE_FILESYSTEM) can export this for you';
        }
        abort(msg);
      },
    });
  }
}

var MAX_UINT8  = (2 **  8) - 1;
var MAX_UINT16 = (2 ** 16) - 1;
var MAX_UINT32 = (2 ** 32) - 1;
var MAX_UINT53 = (2 ** 53) - 1;
var MAX_UINT64 = (2 ** 64) - 1;

var MIN_INT8  = - (2 ** ( 8 - 1));
var MIN_INT16 = - (2 ** (16 - 1));
var MIN_INT32 = - (2 ** (32 - 1));
var MIN_INT53 = - (2 ** (53 - 1));
var MIN_INT64 = - (2 ** (64 - 1));

function checkInt(value, bits, min, max) {
  assert(Number.isInteger(Number(value)), `attempt to write non-integer (${value}) into integer heap`);
  assert(value <= max, `value (${value}) too large to write as ${bits}-bit value`);
  assert(value >= min, `value (${value}) too small to write as ${bits}-bit value`);
}

var checkInt1 = (value) => checkInt(value, 1, 1);
var checkInt8 = (value) => checkInt(value, 8, MIN_INT8, MAX_UINT8);
var checkInt16 = (value) => checkInt(value, 16, MIN_INT16, MAX_UINT16);
var checkInt32 = (value) => checkInt(value, 32, MIN_INT32, MAX_UINT32);
var checkInt53 = (value) => checkInt(value, 53, MIN_INT53, MAX_UINT53);
var checkInt64 = (value) => checkInt(value, 64, MIN_INT64, MAX_UINT64);

// end include: runtime_debug.js
var readyPromiseResolve, readyPromiseReject;

// Memory management
var
/** @type {!Int8Array} */
  HEAP8,
/** @type {!Uint8Array} */
  HEAPU8,
/** @type {!Int16Array} */
  HEAP16,
/** @type {!Uint16Array} */
  HEAPU16,
/** @type {!Int32Array} */
  HEAP32,
/** @type {!Uint32Array} */
  HEAPU32,
/** @type {!Float32Array} */
  HEAPF32,
/** @type {!Float64Array} */
  HEAPF64;

// BigInt64Array type is not correctly defined in closure
var
/** not-@type {!BigInt64Array} */
  HEAP64,
/* BigUint64Array type is not correctly defined in closure
/** not-@type {!BigUint64Array} */
  HEAPU64;

var runtimeInitialized = false;



function updateMemoryViews() {
  var b = wasmMemory.buffer;
  HEAP8 = new Int8Array(b);
  HEAP16 = new Int16Array(b);
  HEAPU8 = new Uint8Array(b);
  HEAPU16 = new Uint16Array(b);
  HEAP32 = new Int32Array(b);
  HEAPU32 = new Uint32Array(b);
  HEAPF32 = new Float32Array(b);
  HEAPF64 = new Float64Array(b);
  HEAP64 = new BigInt64Array(b);
  HEAPU64 = new BigUint64Array(b);
}

// include: memoryprofiler.js
// end include: memoryprofiler.js
// end include: runtime_common.js
assert(globalThis.Int32Array && globalThis.Float64Array && Int32Array.prototype.subarray && Int32Array.prototype.set,
       'JS engine does not provide full typed array support');

function preRun() {
  if (Module['preRun']) {
    if (typeof Module['preRun'] == 'function') Module['preRun'] = [Module['preRun']];
    while (Module['preRun'].length) {
      addOnPreRun(Module['preRun'].shift());
    }
  }
  consumedModuleProp('preRun');
  // Begin ATPRERUNS hooks
  callRuntimeCallbacks(onPreRuns);
  // End ATPRERUNS hooks
}

function initRuntime() {
  assert(!runtimeInitialized);
  runtimeInitialized = true;

  setStackLimits();

  checkStackCookie();

  // No ATINITS hooks

  wasmExports['__wasm_call_ctors']();

  // No ATPOSTCTORS hooks
}

function preMain() {
  checkStackCookie();
  // No ATMAINS hooks
}

function postRun() {
  checkStackCookie();
   // PThreads reuse the runtime from the main thread.

  if (Module['postRun']) {
    if (typeof Module['postRun'] == 'function') Module['postRun'] = [Module['postRun']];
    while (Module['postRun'].length) {
      addOnPostRun(Module['postRun'].shift());
    }
  }
  consumedModuleProp('postRun');

  // Begin ATPOSTRUNS hooks
  callRuntimeCallbacks(onPostRuns);
  // End ATPOSTRUNS hooks
}

/** @param {string|number=} what */
function abort(what) {
  Module['onAbort']?.(what);

  what = 'Aborted(' + what + ')';
  // TODO(sbc): Should we remove printing and leave it up to whoever
  // catches the exception?
  err(what);

  ABORT = true;

  // Use a wasm runtime error, because a JS error might be seen as a foreign
  // exception, which means we'd run destructors on it. We need the error to
  // simply make the program stop.
  // FIXME This approach does not work in Wasm EH because it currently does not assume
  // all RuntimeErrors are from traps; it decides whether a RuntimeError is from
  // a trap or not based on a hidden field within the object. So at the moment
  // we don't have a way of throwing a wasm trap from JS. TODO Make a JS API that
  // allows this in the wasm spec.

  // Suppress closure compiler warning here. Closure compiler's builtin extern
  // definition for WebAssembly.RuntimeError claims it takes no arguments even
  // though it can.
  // TODO(https://github.com/google/closure-compiler/pull/3913): Remove if/when upstream closure gets fixed.
  /** @suppress {checkTypes} */
  var e = new WebAssembly.RuntimeError(what);

  readyPromiseReject?.(e);
  // Throw the error whether or not MODULARIZE is set because abort is used
  // in code paths apart from instantiation where an exception is expected
  // to be thrown when abort is called.
  throw e;
}

// show errors on likely calls to FS when it was not included
var FS = {
  error() {
    abort('Filesystem support (FS) was not included. The problem is that you are using files from JS, but files were not used from C/C++, so filesystem support was not auto-included. You can force-include filesystem support with -sFORCE_FILESYSTEM');
  },
  init() { FS.error() },
  createDataFile() { FS.error() },
  createPreloadedFile() { FS.error() },
  createLazyFile() { FS.error() },
  open() { FS.error() },
  mkdev() { FS.error() },
  registerDevice() { FS.error() },
  analyzePath() { FS.error() },

  ErrnoError() { FS.error() },
};


function createExportWrapper(name, nargs) {
  return (...args) => {
    assert(runtimeInitialized, `native function \`${name}\` called before runtime initialization`);
    var f = wasmExports[name];
    assert(f, `exported native function \`${name}\` not found`);
    // Only assert for too many arguments. Too few can be valid since the missing arguments will be zero filled.
    assert(args.length <= nargs, `native function \`${name}\` called with ${args.length} args but expects ${nargs}`);
    return f(...args);
  };
}

var wasmBinaryFile;

function findWasmBinary() {
  return locateFile('treesheets.wasm');
}

function getBinarySync(file) {
  if (file == wasmBinaryFile && wasmBinary) {
    return new Uint8Array(wasmBinary);
  }
  if (readBinary) {
    return readBinary(file);
  }
  // Throwing a plain string here, even though it not normally adviables since
  // this gets turning into an `abort` in instantiateArrayBuffer.
  throw 'both async and sync fetching of the wasm failed';
}

async function getWasmBinary(binaryFile) {
  // If we don't have the binary yet, load it asynchronously using readAsync.
  if (!wasmBinary) {
    // Fetch the binary using readAsync
    try {
      var response = await readAsync(binaryFile);
      return new Uint8Array(response);
    } catch {
      // Fall back to getBinarySync below;
    }
  }

  // Otherwise, getBinarySync should be able to get it synchronously
  return getBinarySync(binaryFile);
}

async function instantiateArrayBuffer(binaryFile, imports) {
  try {
    var binary = await getWasmBinary(binaryFile);
    var instance = await WebAssembly.instantiate(binary, imports);
    return instance;
  } catch (reason) {
    err(`failed to asynchronously prepare wasm: ${reason}`);

    // Warn on some common problems.
    if (isFileURI(binaryFile)) {
      err(`warning: Loading from a file URI (${binaryFile}) is not supported in most browsers. See https://emscripten.org/docs/getting_started/FAQ.html#how-do-i-run-a-local-webserver-for-testing-why-does-my-program-stall-in-downloading-or-preparing`);
    }
    abort(reason);
  }
}

async function instantiateAsync(binary, binaryFile, imports) {
  if (!binary
     ) {
    try {
      var response = fetch(binaryFile, { credentials: 'same-origin' });
      var instantiationResult = await WebAssembly.instantiateStreaming(response, imports);
      return instantiationResult;
    } catch (reason) {
      // We expect the most common failure cause to be a bad MIME type for the binary,
      // in which case falling back to ArrayBuffer instantiation should work.
      err(`wasm streaming compile failed: ${reason}`);
      err('falling back to ArrayBuffer instantiation');
      // fall back of instantiateArrayBuffer below
    };
  }
  return instantiateArrayBuffer(binaryFile, imports);
}

function getWasmImports() {
  // prepare imports
  var imports = {
    'env': wasmImports,
    'wasi_snapshot_preview1': wasmImports,
  };
  return imports;
}

// Create the wasm instance.
// Receives the wasm imports, returns the exports.
async function createWasm() {
  // Load the wasm module and create an instance of using native support in the JS engine.
  // handle a generated wasm instance, receiving its exports and
  // performing other necessary setup
  /** @param {WebAssembly.Module=} module*/
  function receiveInstance(instance, module) {
    wasmExports = instance.exports;

    assignWasmExports(wasmExports);

    updateMemoryViews();

    return wasmExports;
  }

  // Prefer streaming instantiation if available.
  // Async compilation can be confusing when an error on the page overwrites Module
  // (for example, if the order of elements is wrong, and the one defining Module is
  // later), so we save Module and check it later.
  var trueModule = Module;
  function receiveInstantiationResult(result) {
    // 'result' is a ResultObject object which has both the module and instance.
    // receiveInstance() will swap in the exports (to Module.asm) so they can be called
    assert(Module === trueModule, 'the Module object should not be replaced during async compilation - perhaps the order of HTML elements is wrong?');
    trueModule = null;
    // TODO: Due to Closure regression https://github.com/google/closure-compiler/issues/3193, the above line no longer optimizes out down to the following line.
    // When the regression is fixed, can restore the above PTHREADS-enabled path.
    return receiveInstance(result['instance']);
  }

  var info = getWasmImports();

  // User shell pages can write their own Module.instantiateWasm = function(imports, successCallback) callback
  // to manually instantiate the Wasm module themselves. This allows pages to
  // run the instantiation parallel to any other async startup actions they are
  // performing.
  // Also pthreads and wasm workers initialize the wasm instance through this
  // path.
  if (Module['instantiateWasm']) {
    return new Promise((resolve, reject) => {
      try {
        Module['instantiateWasm'](info, (inst, mod) => {
          resolve(receiveInstance(inst, mod));
        });
      } catch(e) {
        err(`Module.instantiateWasm callback failed with error: ${e}`);
        reject(e);
      }
    });
  }

  wasmBinaryFile ??= findWasmBinary();
  var result = await instantiateAsync(wasmBinary, wasmBinaryFile, info);
  var exports = receiveInstantiationResult(result);
  return exports;
}

// end include: preamble.js

// Begin JS library code


  class ExitStatus {
      name = 'ExitStatus';
      constructor(status) {
        this.message = `Program terminated with exit(${status})`;
        this.status = status;
      }
    }

  var callRuntimeCallbacks = (callbacks) => {
      while (callbacks.length > 0) {
        // Pass the module as the first argument.
        callbacks.shift()(Module);
      }
    };
  var onPostRuns = [];
  var addOnPostRun = (cb) => onPostRuns.push(cb);

  var onPreRuns = [];
  var addOnPreRun = (cb) => onPreRuns.push(cb);


  
    /**
     * @param {number} ptr
     * @param {string} type
     */
  function getValue(ptr, type = 'i8') {
    if (type.endsWith('*')) type = '*';
    switch (type) {
      case 'i1': return HEAP8[ptr];
      case 'i8': return HEAP8[ptr];
      case 'i16': return HEAP16[((ptr)>>1)];
      case 'i32': return HEAP32[((ptr)>>2)];
      case 'i64': return HEAP64[((ptr)>>3)];
      case 'float': return HEAPF32[((ptr)>>2)];
      case 'double': return HEAPF64[((ptr)>>3)];
      case '*': return HEAPU32[((ptr)>>2)];
      default: abort(`invalid type for getValue: ${type}`);
    }
  }

  var noExitRuntime = true;

  var ptrToString = (ptr) => {
      assert(typeof ptr === 'number', `ptrToString expects a number, got ${typeof ptr}`);
      // Convert to 32-bit unsigned value
      ptr >>>= 0;
      return '0x' + ptr.toString(16).padStart(8, '0');
    };

  var setStackLimits = () => {
      var stackLow = _emscripten_stack_get_base();
      var stackHigh = _emscripten_stack_get_end();
      ___set_stack_limits(stackLow, stackHigh);
    };

  
    /**
     * @param {number} ptr
     * @param {number} value
     * @param {string} type
     */
  function setValue(ptr, value, type = 'i8') {
    if (type.endsWith('*')) type = '*';
    switch (type) {
      case 'i1': HEAP8[ptr] = value;checkInt8(value); break;
      case 'i8': HEAP8[ptr] = value;checkInt8(value); break;
      case 'i16': HEAP16[((ptr)>>1)] = value;checkInt16(value); break;
      case 'i32': HEAP32[((ptr)>>2)] = value;checkInt32(value); break;
      case 'i64': HEAP64[((ptr)>>3)] = BigInt(value);checkInt64(value); break;
      case 'float': HEAPF32[((ptr)>>2)] = value; break;
      case 'double': HEAPF64[((ptr)>>3)] = value; break;
      case '*': HEAPU32[((ptr)>>2)] = value; break;
      default: abort(`invalid type for setValue: ${type}`);
    }
  }

  var stackRestore = (val) => __emscripten_stack_restore(val);

  var stackSave = () => _emscripten_stack_get_current();

  var warnOnce = (text) => {
      warnOnce.shown ||= {};
      if (!warnOnce.shown[text]) {
        warnOnce.shown[text] = 1;
        err(text);
      }
    };

  

  var UTF8Decoder = globalThis.TextDecoder && new TextDecoder();
  
  var findStringEnd = (heapOrArray, idx, maxBytesToRead, ignoreNul) => {
      var maxIdx = idx + maxBytesToRead;
      if (ignoreNul) return maxIdx;
      // TextDecoder needs to know the byte length in advance, it doesn't stop on
      // null terminator by itself.
      // As a tiny code save trick, compare idx against maxIdx using a negation,
      // so that maxBytesToRead=undefined/NaN means Infinity.
      while (heapOrArray[idx] && !(idx >= maxIdx)) ++idx;
      return idx;
    };
  
  
    /**
     * Given a pointer 'idx' to a null-terminated UTF8-encoded string in the given
     * array that contains uint8 values, returns a copy of that string as a
     * Javascript String object.
     * heapOrArray is either a regular array, or a JavaScript typed array view.
     * @param {number=} idx
     * @param {number=} maxBytesToRead
     * @param {boolean=} ignoreNul - If true, the function will not stop on a NUL character.
     * @return {string}
     */
  var UTF8ArrayToString = (heapOrArray, idx = 0, maxBytesToRead, ignoreNul) => {
  
      var endPtr = findStringEnd(heapOrArray, idx, maxBytesToRead, ignoreNul);
  
      // When using conditional TextDecoder, skip it for short strings as the overhead of the native call is not worth it.
      if (endPtr - idx > 16 && heapOrArray.buffer && UTF8Decoder) {
        return UTF8Decoder.decode(heapOrArray.subarray(idx, endPtr));
      }
      var str = '';
      while (idx < endPtr) {
        // For UTF8 byte structure, see:
        // http://en.wikipedia.org/wiki/UTF-8#Description
        // https://www.ietf.org/rfc/rfc2279.txt
        // https://tools.ietf.org/html/rfc3629
        var u0 = heapOrArray[idx++];
        if (!(u0 & 0x80)) { str += String.fromCharCode(u0); continue; }
        var u1 = heapOrArray[idx++] & 63;
        if ((u0 & 0xE0) == 0xC0) { str += String.fromCharCode(((u0 & 31) << 6) | u1); continue; }
        var u2 = heapOrArray[idx++] & 63;
        if ((u0 & 0xF0) == 0xE0) {
          u0 = ((u0 & 15) << 12) | (u1 << 6) | u2;
        } else {
          if ((u0 & 0xF8) != 0xF0) warnOnce('Invalid UTF-8 leading byte ' + ptrToString(u0) + ' encountered when deserializing a UTF-8 string in wasm memory to a JS string!');
          u0 = ((u0 & 7) << 18) | (u1 << 12) | (u2 << 6) | (heapOrArray[idx++] & 63);
        }
  
        if (u0 < 0x10000) {
          str += String.fromCharCode(u0);
        } else {
          var ch = u0 - 0x10000;
          str += String.fromCharCode(0xD800 | (ch >> 10), 0xDC00 | (ch & 0x3FF));
        }
      }
      return str;
    };
  
    /**
     * Given a pointer 'ptr' to a null-terminated UTF8-encoded string in the
     * emscripten HEAP, returns a copy of that string as a Javascript String object.
     *
     * @param {number} ptr
     * @param {number=} maxBytesToRead - An optional length that specifies the
     *   maximum number of bytes to read. You can omit this parameter to scan the
     *   string until the first 0 byte. If maxBytesToRead is passed, and the string
     *   at [ptr, ptr+maxBytesToReadr[ contains a null byte in the middle, then the
     *   string will cut short at that byte index.
     * @param {boolean=} ignoreNul - If true, the function will not stop on a NUL character.
     * @return {string}
     */
  var UTF8ToString = (ptr, maxBytesToRead, ignoreNul) => {
      assert(typeof ptr == 'number', `UTF8ToString expects a number (got ${typeof ptr})`);
      return ptr ? UTF8ArrayToString(HEAPU8, ptr, maxBytesToRead, ignoreNul) : '';
    };
  function _JS_AskNumber(titlePtr, msgPtr, def, min, max) {
          var title = UTF8ToString(titlePtr);
          var msg = UTF8ToString(msgPtr);
  
          var input = document.createElement('input');
          input.type = 'number';
          input.value = def;
          input.min = min;
          input.max = max;
          input.style.cssText = 'width:100%;padding:8px;border:1px solid #ccc;border-radius:4px;margin-top:8px;';
  
          var content = document.createElement('div');
          var p = document.createElement('p');
          // XSS SAFETY: Use safe utility instead of innerHTML
          Module._addTextWithLineBreaks(p, msg);
          content.appendChild(p);
          content.appendChild(input);
  
          // Use promise-based approach for proper async handling
          return Asyncify.handleSleep(function(wakeUp) {
              Module._createModal(title, content, [
                  { text: 'Cancel', callback: function() {
                      wakeUp(def);
                  }},
                  { text: 'OK', primary: true, callback: function() {
                      var result = parseFloat(input.value);
                      if (isNaN(result)) result = def;
                      wakeUp(result);
                  }}
              ]);
  
              // Focus input after a brief delay
              setTimeout(function() { input.focus(); }, 100);
          });
      }

  function _JS_AskText(titlePtr, msgPtr, defPtr) {
          var title = UTF8ToString(titlePtr);
          var msg = UTF8ToString(msgPtr);
          var def = UTF8ToString(defPtr);
  
          var resultPtr = null;
          var input = document.createElement('input');
          input.type = 'text';
          input.value = def;
          input.style.cssText = 'width:100%;padding:8px;border:1px solid #ccc;border-radius:4px;margin-top:8px;';
  
          var content = document.createElement('div');
          var p = document.createElement('p');
          // XSS SAFETY: Use safe utility instead of innerHTML
          Module._addTextWithLineBreaks(p, msg);
          content.appendChild(p);
          content.appendChild(input);
  
          // Use promise-based approach for proper async handling
          return Asyncify.handleSleep(function(wakeUp) {
              Module._createModal(title, content, [
                  { text: 'Cancel', callback: function() {
                      var len = 1;
                      resultPtr = _malloc(len);
                      stringToUTF8('', resultPtr, len);
                      wakeUp(resultPtr);
                  }},
                  { text: 'OK', primary: true, callback: function() {
                      var res = input.value || '';
                      var len = lengthBytesUTF8(res) + 1;
                      resultPtr = _malloc(len);
                      stringToUTF8(res, resultPtr, len);
                      wakeUp(resultPtr);
                  }}
              ]);
  
              // Focus input after a brief delay
              setTimeout(function() { input.focus(); }, 100);
          });
      }

  function _JS_DownloadFile(filenamePtr, dataPtr, size) {
          try {
              var filename = Module._stringFromPtr(filenamePtr, 'download.cts');
              var data = new Uint8Array(Module.HEAPU8.buffer, dataPtr, size);
              var blob = new Blob([data], { type: 'application/octet-stream' });
              var url = URL.createObjectURL(blob);
              var a = document.createElement('a');
              a.href = url;
              a.download = filename;
              document.body.appendChild(a);
              a.click();
              document.body.removeChild(a);
              URL.revokeObjectURL(url);
          } catch (e) {
              console.error('Download failed:', e);
              Module._errorHandler.showError('Download Failed',
                  'Unable to download file. Error: ' + e.message);
          }
      }

  function _JS_DrawBitmap(idx, x, y) {
          // Image cache system
          if (!Module.imageCache) Module.imageCache = {};
          if (!Module.imageLoadQueue) Module.imageLoadQueue = {};
  
          var ctx = Module._getCtx();
          var img = Module.imageCache[idx];
          var size = Module._CONSTANTS.PLACEHOLDER_SIZE;
  
          if (img && img.complete && img.naturalWidth > 0) {
              // Image loaded, draw it
              ctx.drawImage(img, x, y);
          } else if (!Module.imageLoadQueue[idx] && !Module.imageCache[idx]) {
              // BUGFIX: Check both queue and cache to prevent race condition
              // Mark as loading IMMEDIATELY to prevent duplicate loads
              Module.imageLoadQueue[idx] = true;
  
              img = new Image();
              var loadingStartTime = Date.now();
  
              img.onload = function() {
                  // Only cache if this is still the expected load operation
                  if (Module.imageLoadQueue[idx]) {
                      Module._cacheManager.addImage(idx, img);
                      delete Module.imageLoadQueue[idx];
  
                      console.log('Image loaded:', idx, 'in', Date.now() - loadingStartTime, 'ms');
  
                      // BUGFIX: Debounce redraws to avoid multiple rapid calls
                      if (Module._WASM_Resize && !Module._pendingRedraw) {
                          Module._pendingRedraw = true;
                          requestAnimationFrame(function() {
                              Module._pendingRedraw = false;
                              var w = Module.canvas.width;
                              var h = Module.canvas.height;
                              Module._WASM_Resize(w, h);
                          });
                      }
                  }
              };
              img.onerror = function() {
                  delete Module.imageLoadQueue[idx];
                  console.warn('Failed to load image:', idx);
                  // Create minimal placeholder image for cache
                  var placeholder = new Image();
                  placeholder.width = size;
                  placeholder.height = size;
                  Module._cacheManager.addImage(idx, placeholder);
              };
              // Set src to start loading (do NOT cache before load completes)
              img.src = 'images/icon' + idx + '.png';
  
              // Draw placeholder while loading
              Module._drawPlaceholder(ctx, x, y, size);
          } else {
              // Loading in progress, draw placeholder
              Module._drawPlaceholder(ctx, x, y, size);
          }
      }

  function _JS_DrawLine(x1, y1, x2, y2) {
          var ctx = Module._getCtx();
          ctx.beginPath();
          ctx.moveTo(x1, y1);
          ctx.lineTo(x2, y2);
          ctx.stroke();
      }

  function _JS_DrawRectangle(x, y, w, h) {
          var ctx = Module._getCtx();
          ctx.fillRect(x, y, w, h);
          ctx.strokeRect(x, y, w, h);
      }

  function _JS_DrawRoundedRectangle(x, y, w, h, radius) {
          var ctx = Module._getCtx();
          ctx.beginPath();
          if (ctx.roundRect) {
              ctx.roundRect(x, y, w, h, radius);
          } else {
              // POLYFILL: Manual roundRect implementation for older browsers
              // This ensures consistent UI across all browsers
              ctx.moveTo(x + radius, y);
              ctx.lineTo(x + w - radius, y);
              ctx.quadraticCurveTo(x + w, y, x + w, y + radius);
              ctx.lineTo(x + w, y + h - radius);
              ctx.quadraticCurveTo(x + w, y + h, x + w - radius, y + h);
              ctx.lineTo(x + radius, y + h);
              ctx.quadraticCurveTo(x, y + h, x, y + h - radius);
              ctx.lineTo(x, y + radius);
              ctx.quadraticCurveTo(x, y, x + radius, y);
              ctx.closePath();
          }
          ctx.fill();
          ctx.stroke();
      }

  function _JS_DrawText(str, x, y) {
          if (!Module._validation.isValidPointer(str)) return;
          var ctx = Module._getCtx();
          var s = UTF8ToString(str);
          // Apply stored text foreground color before drawing
          // This ensures brush color changes don't affect text color
          ctx.fillStyle = Module._textForegroundColor;
          console.log('DrawText:', s, 'at', x, y, 'font:', ctx.font, 'fill:', ctx.fillStyle);
          ctx.fillText(s, x, y);
      }

  function _JS_GetCharHeight() {
          var ctx = Module._getCtx();
          // Use TextMetrics to get accurate font height
          var metrics = ctx.measureText('M'); // Use 'M' as it's typically the tallest character
          return Module._getFontHeight(metrics, ctx);
      }

  function _JS_GetClipboardText() {
          // Modern Clipboard API with Asyncify
          // MEMORY OWNERSHIP: Returns a malloc'd pointer that the C++ caller MUST free using Module._free()
          // Returns 0 (null pointer) on failure or if clipboard is empty
          if (navigator.clipboard && navigator.clipboard.readText) {
              return Asyncify.handleSleep(function(wakeUp) {
                  navigator.clipboard.readText()
                      .then(function(text) {
                          var len = lengthBytesUTF8(text) + 1;
                          var ptr = _malloc(len);
                          if (!ptr) {
                              console.error('Failed to allocate memory for clipboard text');
                              wakeUp(0);
                              return;
                          }
                          stringToUTF8(text, ptr, len);
                          // CRITICAL: Caller must free this pointer after use!
                          wakeUp(ptr);
                      })
                      .catch(function(err) {
                          console.warn('Clipboard read failed:', err);
                          wakeUp(0);
                      });
              });
          } else {
              // Fallback: return empty string for browsers without clipboard API
              console.warn('Clipboard API not available');
              return 0;
          }
      }

  function _JS_GetLastFileSize() {
          // Returns size of last file read by JS_ReadFile
          // WARNING: Must be called IMMEDIATELY after JS_ReadFile due to global state
          return Module._lastReadFileSize || 0;
      }

  function _JS_GetSelectedFont() {
          return Module._lastSelectedFont || 0;
      }

  function _JS_GetSelectedFontSize() {
          return Module._lastSelectedFontSize || 12;
      }

  function _JS_GetTextWidth(str) {
          var ctx = Module._getCtx();
          var s = UTF8ToString(str);
          return Math.ceil(ctx.measureText(s).width);
      }

  function _JS_InitInput() {
          // REFACTORED: Broken down from 184 lines into focused, testable functions
          // Prevent duplicate initialization (avoids event listener accumulation)
          if (Module._inputInitialized) {
              console.warn('JS_InitInput: Already initialized, skipping duplicate call');
              return;
          }
          Module._inputInitialized = true;
  
          var canvas = Module.canvas;
  
          // Initialize each input subsystem
          Module._initMouseEvents(canvas);
          Module._initWheelEvents(canvas);
          Module._initTouchEvents(canvas);
          Module._initKeyboardEvents();
          Module._initResizeHandler(canvas);
  
          // Initialize feature systems
          // NOTE: These are stubbed out for MVP - internal library functions aren't
          // automatically exposed on Module in Emscripten. These features (auto-save,
          // dark mode, themes, templates, etc.) can be enabled later with proper
          // architecture refactoring.
          console.log('Skipping polished/power/complete features for MVP - basic functionality enabled');
          // Module._initPolishedFeatures();
          // Module._initPowerFeatures();
          // Module._initCompleteFeatures();
  
          // Save session on page unload (disabled for MVP - _sessionRecovery not on Module)
          // window.addEventListener('unload', function() {
          //     if (Module._sessionRecovery) {
          //         Module._sessionRecovery.save();
          //     }
          // });
      }

  function _JS_IsDarkMode() {
          return window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? 1 : 0;
      }

  function _JS_LaunchBrowser(urlPtr) {
          try {
              var url = Module._stringFromPtr(urlPtr, '');
              if (!url) {
                  console.warn('Empty URL provided to JS_LaunchBrowser');
                  return;
              }
              var win = window.open(url, '_blank');
              if (!win) {
                  Module._errorHandler.showError('Popup Blocked',
                      'Your browser blocked the popup. Please allow popups for this site to open links in new tabs.');
              }
          } catch (e) {
              console.error('Failed to open URL:', e);
              Module._errorHandler.showError('Navigation Failed',
                  'Unable to open link. Error: ' + e.message);
          }
      }

  function _JS_MenuBar_Append(menuId, titlePtr) {
          var title = UTF8ToString(titlePtr);
          var menu = Module.menus[menuId];
          var menubar = document.getElementById('menubar');
          if (!menubar) {
              menubar = document.createElement('div');
              menubar.id = 'menubar';
              menubar.style.backgroundColor = '#e0e0e0';
              menubar.style.padding = '5px';
              menubar.style.display = 'flex';
              menubar.style.gap = '10px';
              menubar.setAttribute('role', 'menubar');
              menubar.setAttribute('aria-label', 'Main menu');
              if (Module.canvas && Module.canvas.parentNode) {
                  Module.canvas.parentNode.insertBefore(menubar, Module.canvas);
              } else {
                  document.body.prepend(menubar);
              }
          }
  
          var btn = document.createElement('button');
          btn.textContent = title.replace('&', '');
          btn.style.border = 'none';
          btn.style.background = 'none';
          btn.style.cursor = 'pointer';
          btn.style.fontSize = '14px';
          btn.setAttribute('role', 'menuitem');
          btn.setAttribute('aria-haspopup', 'true');
          btn.setAttribute('aria-label', title.replace('&', ''));
  
          var openMenu = function(e) {
              var oldPopup = document.getElementById('menu-popup');
              if (oldPopup) oldPopup.remove();
  
              var popup = document.createElement('div');
              popup.id = 'menu-popup';
              popup.style.position = 'absolute';
              popup.style.left = e.pageX + 'px';
              popup.style.top = (e.pageY + 20) + 'px';
              popup.style.backgroundColor = 'white';
              popup.style.border = '1px solid #ccc';
              popup.style.boxShadow = '2px 2px 5px rgba(0,0,0,0.2)';
              popup.style.zIndex = 1000;
              popup.setAttribute('role', 'menu');
              popup.setAttribute('aria-label', title.replace('&', '') + ' menu');
  
              var menuItems = [];
              menu.items.forEach(function(item, index) {
                  if (item.type == 3) { // Separator
                      var sep = document.createElement('hr');
                      sep.style.margin = '2px 0';
                      sep.setAttribute('role', 'separator');
                      popup.appendChild(sep);
                      return;
                  }
                  var itemDiv = document.createElement('div');
                  itemDiv.textContent = item.text.replace(/&/g, '').split('\t')[0];
                  itemDiv.style.padding = '5px 15px';
                  itemDiv.style.cursor = 'pointer';
                  itemDiv.setAttribute('role', 'menuitem');
                  itemDiv.setAttribute('tabindex', '0');
                  itemDiv.setAttribute('aria-label', item.text.replace(/&/g, '').split('\t')[0]);
                  itemDiv.onmouseover = function() { itemDiv.style.backgroundColor = '#eee'; };
                  itemDiv.onmouseout = function() { itemDiv.style.backgroundColor = 'white'; };
                  itemDiv.onclick = function() {
                      Module._WASM_Action(item.id);
                      popup.remove();
                      document.removeEventListener('keydown', menuKeyHandler);
                      window.removeEventListener('mousedown', closeHandler);
                  };
                  menuItems.push(itemDiv);
                  popup.appendChild(itemDiv);
              });
  
              document.body.appendChild(popup);
  
              // Keyboard navigation for menu
              var currentIndex = -1;
              var menuKeyHandler = function(ev) {
                  // EDGE CASE: Prevent divide by zero if menu has no items
                  if (menuItems.length === 0) return;
  
                  if (ev.key === 'ArrowDown') {
                      ev.preventDefault();
                      currentIndex = (currentIndex + 1) % menuItems.length;
                      menuItems[currentIndex].focus();
                      menuItems[currentIndex].style.backgroundColor = '#eee';
                  } else if (ev.key === 'ArrowUp') {
                      ev.preventDefault();
                      currentIndex = currentIndex <= 0 ? menuItems.length - 1 : currentIndex - 1;
                      menuItems[currentIndex].focus();
                      menuItems[currentIndex].style.backgroundColor = '#eee';
                  } else if (ev.key === 'Enter' || ev.key === ' ') {
                      ev.preventDefault();
                      if (currentIndex >= 0) menuItems[currentIndex].click();
                  } else if (ev.key === 'Escape') {
                      ev.preventDefault();
                      popup.remove();
                      document.removeEventListener('keydown', menuKeyHandler);
                      window.removeEventListener('mousedown', closeHandler);
                      btn.focus();
                  }
              };
              document.addEventListener('keydown', menuKeyHandler);
  
              // Focus first item
              if (menuItems.length > 0) {
                  currentIndex = 0;
                  menuItems[0].focus();
                  menuItems[0].style.backgroundColor = '#eee';
              }
  
              var closeHandler = function(ev) {
                  if (ev.target != btn && !popup.contains(ev.target)) {
                      popup.remove();
                      document.removeEventListener('keydown', menuKeyHandler);
                      window.removeEventListener('mousedown', closeHandler);
                  }
              };
              setTimeout(function() { window.addEventListener('mousedown', closeHandler); }, 0);
          };
  
          btn.onclick = openMenu;
          btn.onkeydown = function(e) {
              if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  openMenu(e);
              }
          };
          menubar.appendChild(btn);
      }

  function _JS_Menu_Append(parentId, id, textPtr, helpPtr, type, checked) {
          var text = UTF8ToString(textPtr);
          if (Module.menus[parentId]) {
              Module.menus[parentId].items.push({id: id, text: text, type: type, checked: checked});
          }
      }

  function _JS_Menu_AppendSubMenu(parentId, submenuId, textPtr, helpPtr) {
          var text = UTF8ToString(textPtr);
          if (Module.menus[parentId]) {
              Module.menus[parentId].items.push({id: submenuId, text: text, type: 4, submenu: true});
          }
      }

  function _JS_Menu_Check(menuId, itemId, checked) {
          if (Module.menus[menuId]) {
              var items = Module.menus[menuId].items;
              for (var i = 0; i < items.length; i++) {
                  if (items[i].id === itemId) {
                      items[i].checked = checked;
                      break;
                  }
              }
          }
      }

  function _JS_Menu_Create(id, titlePtr) {
          var title = UTF8ToString(titlePtr);
          if (!Module.menus) Module.menus = {};
          Module.menus[id] = { title: title, items: [] };
      }

  function _JS_PickColor(defaultColor) {
          // Use color utility for hex conversion
          var hexColor = Module._colorUtils.toHex(defaultColor);
  
          var content = document.createElement('div');
          var input = document.createElement('input');
          input.type = 'color';
          input.value = hexColor;
          input.style.cssText = 'width:100%;height:100px;border:1px solid #ccc;border-radius:4px;cursor:pointer;';
          content.appendChild(input);
  
          // Use promise-based approach for proper async handling
          return Asyncify.handleSleep(function(wakeUp) {
              Module._createModal('Choose Color', content, [
                  { text: 'Cancel', callback: function() {
                      wakeUp(defaultColor);
                  }},
                  { text: 'OK', primary: true, callback: function() {
                      var result = Module._colorUtils.fromHex(input.value);
                      wakeUp(result);
                  }}
              ]);
  
              // Focus input after a brief delay
              setTimeout(function() { input.focus(); }, 100);
          });
      }

  function _JS_ReadFile(filenamePtr) {
          // MEMORY OWNERSHIP: Returns a malloc'd pointer that the C++ caller MUST free using Module._free()
          // Returns 0 (null pointer) if file not found or on error
          // IMPORTANT: Caller must call JS_GetLastFileSize IMMEDIATELY after JS_ReadFile to get the size
          // WARNING: _lastReadFileSize is global state - concurrent calls will have race conditions
          var filename = UTF8ToString(filenamePtr);
          var bytes = Module._VFS.loadFile(filename);
  
          if (!bytes || bytes.length === 0) {
              console.warn('File not found in VFS:', filename);
              Module._lastReadFileSize = 0;
              return 0; // Return null pointer if file not found
          }
  
          // Allocate memory and copy file data
          var ptr = _malloc(bytes.length);
          if (!ptr) {
              console.error('Failed to allocate memory for file read');
              Module._lastReadFileSize = 0;
              return 0;
          }
  
          Module.HEAPU8.set(bytes, ptr);
  
          // Store size for caller to retrieve
          // NOTE: Global state - caller MUST call JS_GetLastFileSize immediately
          Module._lastReadFileSize = bytes.length;
  
          return ptr; // CRITICAL: Caller must free this pointer!
      }

  function _JS_SelectFont(defaultFontPtr, defaultSize) {
          var fonts = [
              'Arial', 'Helvetica', 'Times New Roman', 'Georgia', 'Verdana',
              'Courier New', 'Consolas', 'Monaco', 'Lucida Console',
              'Trebuchet MS', 'Tahoma', 'Impact', 'Comic Sans MS'
          ];
          var defaultFont = UTF8ToString(defaultFontPtr);
  
          // Build font list string
          var txt = "Select Font:\n";
          fonts.forEach(function(f, i) { txt += i + ": " + f + "\n"; });
          txt += "\nEnter font number (or -1 to cancel):";
  
          var fontIdx = parseInt(prompt(txt, "0"));
          if (isNaN(fontIdx) || fontIdx < 0 || fontIdx >= fonts.length) return 0; // Cancelled
  
          var sizeStr = prompt("Enter font size (8-72):", defaultSize.toString());
          var size = parseInt(sizeStr);
          if (isNaN(size) || size < 8 || size > 72) return 0; // Cancelled
  
          // Store selected font name and size in Module for retrieval
          var selectedFont = fonts[fontIdx];
          var len = lengthBytesUTF8(selectedFont) + 1;
          var ptr = _malloc(len);
          stringToUTF8(selectedFont, ptr, len);
          Module._lastSelectedFont = ptr;
          Module._lastSelectedFontSize = size;
  
          return 1; // Success
      }

  function _JS_SetBrush(brushType) {
          var ctx = Module._getCtx();
          // Use object lookup instead of switch statement
          var brushColor = Module._brushStyles[brushType] || Module._brushStyles[1]; // Default to BRUSH_WHITE
          // CLEANED: Removed redundant Module._currentBrush assignment - not used anywhere
          ctx.fillStyle = brushColor;
      }

  function _JS_SetBrushColor(c) {
          var ctx = Module._getCtx();
          ctx.fillStyle = Module._colorUtils.toRGBString(c);
      }

  function _JS_SetClipboardText(textPtr) {
          var text = UTF8ToString(textPtr);
  
          // Modern Clipboard API
          if (navigator.clipboard && navigator.clipboard.writeText) {
              navigator.clipboard.writeText(text).catch(function(err) {
                  console.warn('Clipboard write failed, trying fallback:', err);
                  Module._clipboardFallbackCopy(text);
              });
          } else {
              // Legacy fallback for older browsers
              Module._clipboardFallbackCopy(text);
          }
      }

  function __tsHelpers() {}
  function _JS_SetFont(size, stylebits) {
          var ctx = Module._getCtx();
          var constants = Module._CONSTANTS;
  
          // Validate and clamp font size
          size = Module._validation.clampFontSize(size);
  
          var font = '';
          if (stylebits & constants.FONT_ITALIC) font += 'italic ';
          if (stylebits & constants.FONT_BOLD) font += 'bold ';
          font += size + 'px ';
          if (stylebits & constants.FONT_MONOSPACE) font += 'monospace';
          else font += 'sans-serif';
          ctx.font = font;
      }

  function _JS_SetPen(penType) {
          var ctx = Module._getCtx();
          // Use object lookup instead of switch statement
          var style = Module._penStyles[penType] || Module._penStyles[6]; // Default to PEN_BLACK
          ctx.strokeStyle = style.strokeStyle;
          ctx.lineWidth = style.lineWidth;
          ctx.setLineDash(style.lineDash);
      }

  function _JS_SetPenColor(c) {
          var ctx = Module._getCtx();
          ctx.strokeStyle = Module._colorUtils.toRGBString(c);
      }

  function _JS_SetTextBackground(c) {
          // NOTE: Text background color is not implemented in the canvas 2D API.
          // Canvas fillText only supports foreground color via fillStyle.
          // If text backgrounds are needed, they must be drawn separately using fillRect
          // before rendering the text. The C++ code does not currently request this.
      }

  function _JS_SetTextForeground(c) {
          // Store text color separately - don't set ctx.fillStyle here
          // JS_DrawText will apply this color before drawing
          Module._textForegroundColor = Module._colorUtils.toRGBString(c);
      }

  function _JS_ShowMessage(titlePtr, msgPtr) {
          var title = UTF8ToString(titlePtr);
          var msg = UTF8ToString(msgPtr);
  
          // Security: Create safe content with textContent
          var content = document.createElement('div');
          var p = document.createElement('p');
          // XSS SAFETY: Use safe utility instead of innerHTML
          Module._addTextWithLineBreaks(p, msg);
          content.appendChild(p);
  
          Module._createModal(title, content, [
              { text: 'OK', primary: true }
          ]);
      }

  function _JS_SingleChoice(titlePtr, msgPtr, choicesJsonPtr) {
          var title = UTF8ToString(titlePtr);
          var msg = UTF8ToString(msgPtr);
  
          // Add error handling for JSON parsing
          var choices;
          try {
              choices = JSON.parse(UTF8ToString(choicesJsonPtr));
          } catch (e) {
              console.error('JS_SingleChoice: Invalid JSON for choices:', e);
              Module._errorHandler.showError('Invalid Data',
                  'The application provided invalid data for this dialog. This may be a bug. Error: ' + e.message);
              return 0;
          }
  
          var content = document.createElement('div');
          var p = document.createElement('p');
          // XSS SAFETY: Use safe utility instead of innerHTML
          Module._addTextWithLineBreaks(p, msg);
          content.appendChild(p);
  
          var select = document.createElement('select');
          select.style.cssText = 'width:100%;padding:8px;border:1px solid #ccc;border-radius:4px;margin-top:8px;';
          choices.forEach(function(choice, i) {
              var option = document.createElement('option');
              option.value = i;
              option.text = choice;
              select.appendChild(option);
          });
          content.appendChild(select);
  
          // Use promise-based approach for proper async handling
          return Asyncify.handleSleep(function(wakeUp) {
              Module._createModal(title, content, [
                  { text: 'Cancel', callback: function() {
                      wakeUp(0);
                  }},
                  { text: 'OK', primary: true, callback: function() {
                      var result = parseInt(select.value, 10);
                      if (isNaN(result)) result = 0;
                      wakeUp(result);
                  }}
              ]);
  
              // Focus select after a brief delay
              setTimeout(function() { select.focus(); }, 100);
          });
      }

  function _JS_Toolbar_AddDropdown(id, width, choicesJsonPtr) {
          var toolbar = document.getElementById('toolbar');
          if(!toolbar) return;
  
          // Add error handling for JSON parsing
          var choices;
          try {
              choices = JSON.parse(UTF8ToString(choicesJsonPtr));
          } catch (e) {
              console.error('JS_Toolbar_AddDropdown: Invalid JSON for choices:', e);
              return;
          }
  
          var sel = document.createElement('select');
          sel.style.width = width + 'px';
          choices.forEach(function(c, i) {
              var opt = document.createElement('option');
              opt.value = i;
              opt.text = c.toString();
              sel.appendChild(opt);
          });
          sel.onchange = function() {
               Module._WASM_Action(id);
          };
          toolbar.appendChild(sel);
      }

  function _JS_Toolbar_AddInput(id, width, defPtr) {
          var toolbar = document.getElementById('toolbar');
          if(!toolbar) return;
          var inp = document.createElement('input');
          inp.type = 'text';
          inp.value = UTF8ToString(defPtr);
          inp.style.width = width + 'px';
          inp.onkeydown = function(e) {
              if(e.key === 'Enter') {
                   Module._WASM_Action(id);
              }
          };
          toolbar.appendChild(inp);
      }

  function _JS_Toolbar_AddLabel(labelPtr) {
          var toolbar = document.getElementById('toolbar');
          if(!toolbar) return;
          var span = document.createElement('span');
          span.textContent = UTF8ToString(labelPtr);
          span.style.fontSize = '12px';
          toolbar.appendChild(span);
      }

  function _JS_Toolbar_AddSeparator() {
          var toolbar = document.getElementById('toolbar');
          if(!toolbar) return;
          var sep = document.createElement('div');
          sep.style.width = '1px';
          sep.style.height = '20px';
          sep.style.backgroundColor = '#999';
          sep.style.margin = '0 5px';
          toolbar.appendChild(sep);
      }

  function _JS_Toolbar_AddTool(id, labelPtr, iconPtr) {
          var toolbar = document.getElementById('toolbar');
          if(!toolbar) return;
          var label = UTF8ToString(labelPtr);
          var iconPath = iconPtr ? UTF8ToString(iconPtr) : null;
  
          var btn = document.createElement('button');
          btn.title = label;
          btn.style.cssText = 'padding:4px 8px;border:1px solid #ccc;border-radius:3px;background:#fff;cursor:pointer;display:flex;align-items:center;gap:4px;';
          btn.setAttribute('aria-label', label);
          btn.onmouseover = function() { btn.style.background = '#e8e8e8'; };
          btn.onmouseout = function() { btn.style.background = '#fff'; };
  
          // Try to load icon if path provided
          if (iconPath) {
              var img = new Image();
              img.src = iconPath;
              img.width = 16;
              img.height = 16;
              img.style.display = 'inline-block';
              img.onerror = function() {
                  // Fallback to text label if icon fails
                  btn.textContent = label.split(' (')[0];
              };
              img.onload = function() {
                  btn.appendChild(img);
                  var span = document.createElement('span');
                  span.textContent = label.split(' (')[0];
                  span.style.fontSize = '12px';
                  btn.appendChild(span);
              };
          } else {
              btn.textContent = label.split(' (')[0];
          }
  
          btn.onclick = function() { Module._WASM_Action(id); };
          toolbar.appendChild(btn);
      }

  function _JS_Toolbar_Create() {
          var toolbar = document.getElementById('toolbar');
          if (!toolbar) {
              toolbar = document.createElement('div');
              toolbar.id = 'toolbar';
              toolbar.style.backgroundColor = '#f0f0f0';
              toolbar.style.padding = '5px';
              toolbar.style.display = 'flex';
              toolbar.style.flexWrap = 'wrap';
              toolbar.style.gap = '5px';
              toolbar.style.alignItems = 'center';
              toolbar.style.borderBottom = '1px solid #ccc';
              toolbar.setAttribute('role', 'toolbar');
              toolbar.setAttribute('aria-label', 'TreeSheets Toolbar');
              var menubar = document.getElementById('menubar');
              if (menubar && menubar.parentNode) {
                  menubar.parentNode.insertBefore(toolbar, menubar.nextSibling);
              } else if (Module.canvas && Module.canvas.parentNode) {
                  Module.canvas.parentNode.insertBefore(toolbar, Module.canvas);
              } else {
                  document.body.prepend(toolbar);
              }
          }
      }

  function _JS_TriggerUpload() {
          var input = document.createElement('input');
          input.type = 'file';
          input.accept = '.cts';
          input.onchange = function(e) {
              var file = e.target.files[0];
              if (!file) {
                  // Clean up input element
                  if (input.parentNode) input.parentNode.removeChild(input);
                  return;
              }
  
              // Security: Check file size
              var maxSize = Module._CONSTANTS.MAX_FILE_SIZE_BYTES;
              if (file.size > maxSize) {
                  Module._createModal('Error', '<p>File too large. Maximum size is ' +
                      Math.floor(maxSize / (1024 * 1024)) + 'MB.</p>', [
                      { text: 'OK', primary: true }
                  ]);
                  // Clean up input element
                  if (input.parentNode) input.parentNode.removeChild(input);
                  return;
              }
  
              var reader = new FileReader();
              reader.onload = function(evt) {
                  try {
                      var arrayBuffer = evt.target.result;
                      var uint8Array = new Uint8Array(arrayBuffer);
  
                      // Check malloc succeeded
                      var ptr = Module._malloc(uint8Array.length);
                      if (!ptr) {
                          Module._createModal('Error', '<p>Failed to allocate memory for file.</p>', [
                              { text: 'OK', primary: true }
                          ]);
                          return;
                      }
                      Module.HEAPU8.set(uint8Array, ptr);
  
                      var nameLen = lengthBytesUTF8(file.name) + 1;
                      var namePtr = Module._malloc(nameLen);
                      if (!namePtr) {
                          Module._free(ptr);
                          Module._createModal('Error', '<p>Failed to allocate memory for filename.</p>', [
                              { text: 'OK', primary: true }
                          ]);
                          return;
                      }
                      stringToUTF8(file.name, namePtr, nameLen);
  
                      // Save file to VFS for later reading
                      Module._VFS.saveFile(file.name, uint8Array);
  
                      // Add to recent files
                      if (Module._recentFiles) {
                          Module._recentFiles.add(file.name);
                      }
  
                      // Mark as dirty for auto-save
                      if (Module._autoSave) {
                          Module._autoSave.markDirty();
                      }
  
                      Module._WASM_FileLoaded(namePtr, ptr, uint8Array.length);
  
                      Module._free(ptr);
                      Module._free(namePtr);
                  } catch (err) {
                      console.error('Error processing uploaded file:', err);
                      Module._createModal('Error', '<p>Failed to process file: ' + err.message + '</p>', [
                          { text: 'OK', primary: true }
                      ]);
                  } finally {
                      // Clean up input element
                      if (input.parentNode) input.parentNode.removeChild(input);
                  }
              };
              reader.onerror = function(err) {
                  console.error('FileReader error:', err);
                  Module._createModal('Error', '<p>Failed to read file.</p>', [
                      { text: 'OK', primary: true }
                  ]);
                  // Clean up input element
                  if (input.parentNode) input.parentNode.removeChild(input);
              };
              reader.readAsArrayBuffer(file);
          };
          input.click();
      }

  var ___assert_fail = (condition, filename, line, func) =>
      abort(`Assertion failed: ${UTF8ToString(condition)}, at: ` + [filename ? UTF8ToString(filename) : 'unknown filename', line, func ? UTF8ToString(func) : 'unknown function']);

  class ExceptionInfo {
      // excPtr - Thrown object pointer to wrap. Metadata pointer is calculated from it.
      constructor(excPtr) {
        this.excPtr = excPtr;
        this.ptr = excPtr - 24;
      }
  
      set_type(type) {
        HEAPU32[(((this.ptr)+(4))>>2)] = type;
      }
  
      get_type() {
        return HEAPU32[(((this.ptr)+(4))>>2)];
      }
  
      set_destructor(destructor) {
        HEAPU32[(((this.ptr)+(8))>>2)] = destructor;
      }
  
      get_destructor() {
        return HEAPU32[(((this.ptr)+(8))>>2)];
      }
  
      set_caught(caught) {
        caught = caught ? 1 : 0;
        HEAP8[(this.ptr)+(12)] = caught;checkInt8(caught);
      }
  
      get_caught() {
        return HEAP8[(this.ptr)+(12)] != 0;
      }
  
      set_rethrown(rethrown) {
        rethrown = rethrown ? 1 : 0;
        HEAP8[(this.ptr)+(13)] = rethrown;checkInt8(rethrown);
      }
  
      get_rethrown() {
        return HEAP8[(this.ptr)+(13)] != 0;
      }
  
      // Initialize native structure fields. Should be called once after allocated.
      init(type, destructor) {
        this.set_adjusted_ptr(0);
        this.set_type(type);
        this.set_destructor(destructor);
      }
  
      set_adjusted_ptr(adjustedPtr) {
        HEAPU32[(((this.ptr)+(16))>>2)] = adjustedPtr;
      }
  
      get_adjusted_ptr() {
        return HEAPU32[(((this.ptr)+(16))>>2)];
      }
    }
  
  var exceptionLast = 0;
  
  var uncaughtExceptionCount = 0;
  var ___cxa_throw = (ptr, type, destructor) => {
      var info = new ExceptionInfo(ptr);
      // Initialize ExceptionInfo content after it was allocated in __cxa_allocate_exception.
      info.init(type, destructor);
      exceptionLast = ptr;
      uncaughtExceptionCount++;
      assert(false, 'Exception thrown, but exception catching is not enabled. Compile with -sNO_DISABLE_EXCEPTION_CATCHING or -sEXCEPTION_CATCHING_ALLOWED=[..] to catch.');
    };

  
  
  var ___handle_stack_overflow = (requested) => {
      var base = _emscripten_stack_get_base();
      var end = _emscripten_stack_get_end();
      abort(`stack overflow (Attempt to set SP to ${ptrToString(requested)}` +
            `, with stack limits [${ptrToString(end)} - ${ptrToString(base)}` +
            ']). If you require more stack space build with -sSTACK_SIZE=<bytes>');
    };

  var __abort_js = () =>
      abort('native code called abort()');

  var stringToUTF8Array = (str, heap, outIdx, maxBytesToWrite) => {
      assert(typeof str === 'string', `stringToUTF8Array expects a string (got ${typeof str})`);
      // Parameter maxBytesToWrite is not optional. Negative values, 0, null,
      // undefined and false each don't write out any bytes.
      if (!(maxBytesToWrite > 0))
        return 0;
  
      var startIdx = outIdx;
      var endIdx = outIdx + maxBytesToWrite - 1; // -1 for string null terminator.
      for (var i = 0; i < str.length; ++i) {
        // For UTF8 byte structure, see http://en.wikipedia.org/wiki/UTF-8#Description
        // and https://www.ietf.org/rfc/rfc2279.txt
        // and https://tools.ietf.org/html/rfc3629
        var u = str.codePointAt(i);
        if (u <= 0x7F) {
          if (outIdx >= endIdx) break;
          heap[outIdx++] = u;
        } else if (u <= 0x7FF) {
          if (outIdx + 1 >= endIdx) break;
          heap[outIdx++] = 0xC0 | (u >> 6);
          heap[outIdx++] = 0x80 | (u & 63);
        } else if (u <= 0xFFFF) {
          if (outIdx + 2 >= endIdx) break;
          heap[outIdx++] = 0xE0 | (u >> 12);
          heap[outIdx++] = 0x80 | ((u >> 6) & 63);
          heap[outIdx++] = 0x80 | (u & 63);
        } else {
          if (outIdx + 3 >= endIdx) break;
          if (u > 0x10FFFF) warnOnce('Invalid Unicode code point ' + ptrToString(u) + ' encountered when serializing a JS string to a UTF-8 string in wasm memory! (Valid unicode code points should be in range 0-0x10FFFF).');
          heap[outIdx++] = 0xF0 | (u >> 18);
          heap[outIdx++] = 0x80 | ((u >> 12) & 63);
          heap[outIdx++] = 0x80 | ((u >> 6) & 63);
          heap[outIdx++] = 0x80 | (u & 63);
          // Gotcha: if codePoint is over 0xFFFF, it is represented as a surrogate pair in UTF-16.
          // We need to manually skip over the second code unit for correct iteration.
          i++;
        }
      }
      // Null-terminate the pointer to the buffer.
      heap[outIdx] = 0;
      return outIdx - startIdx;
    };
  var stringToUTF8 = (str, outPtr, maxBytesToWrite) => {
      assert(typeof maxBytesToWrite == 'number', 'stringToUTF8(str, outPtr, maxBytesToWrite) is missing the third parameter that specifies the length of the output buffer!');
      return stringToUTF8Array(str, HEAPU8, outPtr, maxBytesToWrite);
    };
  
  var lengthBytesUTF8 = (str) => {
      var len = 0;
      for (var i = 0; i < str.length; ++i) {
        // Gotcha: charCodeAt returns a 16-bit word that is a UTF-16 encoded code
        // unit, not a Unicode code point of the character! So decode
        // UTF16->UTF32->UTF8.
        // See http://unicode.org/faq/utf_bom.html#utf16-3
        var c = str.charCodeAt(i); // possibly a lead surrogate
        if (c <= 0x7F) {
          len++;
        } else if (c <= 0x7FF) {
          len += 2;
        } else if (c >= 0xD800 && c <= 0xDFFF) {
          len += 4; ++i;
        } else {
          len += 3;
        }
      }
      return len;
    };
  var __tzset_js = (timezone, daylight, std_name, dst_name) => {
      // TODO: Use (malleable) environment variables instead of system settings.
      var currentYear = new Date().getFullYear();
      var winter = new Date(currentYear, 0, 1);
      var summer = new Date(currentYear, 6, 1);
      var winterOffset = winter.getTimezoneOffset();
      var summerOffset = summer.getTimezoneOffset();
  
      // Local standard timezone offset. Local standard time is not adjusted for
      // daylight savings.  This code uses the fact that getTimezoneOffset returns
      // a greater value during Standard Time versus Daylight Saving Time (DST).
      // Thus it determines the expected output during Standard Time, and it
      // compares whether the output of the given date the same (Standard) or less
      // (DST).
      var stdTimezoneOffset = Math.max(winterOffset, summerOffset);
  
      // timezone is specified as seconds west of UTC ("The external variable
      // `timezone` shall be set to the difference, in seconds, between
      // Coordinated Universal Time (UTC) and local standard time."), the same
      // as returned by stdTimezoneOffset.
      // See http://pubs.opengroup.org/onlinepubs/009695399/functions/tzset.html
      HEAPU32[((timezone)>>2)] = stdTimezoneOffset * 60;
  
      HEAP32[((daylight)>>2)] = Number(winterOffset != summerOffset);checkInt32(Number(winterOffset != summerOffset));
  
      var extractZone = (timezoneOffset) => {
        // Why inverse sign?
        // Read here https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date/getTimezoneOffset
        var sign = timezoneOffset >= 0 ? "-" : "+";
  
        var absOffset = Math.abs(timezoneOffset)
        var hours = String(Math.floor(absOffset / 60)).padStart(2, "0");
        var minutes = String(absOffset % 60).padStart(2, "0");
  
        return `UTC${sign}${hours}${minutes}`;
      }
  
      var winterName = extractZone(winterOffset);
      var summerName = extractZone(summerOffset);
      assert(winterName);
      assert(summerName);
      assert(lengthBytesUTF8(winterName) <= 16, `timezone name truncated to fit in TZNAME_MAX (${winterName})`);
      assert(lengthBytesUTF8(summerName) <= 16, `timezone name truncated to fit in TZNAME_MAX (${summerName})`);
      if (summerOffset < winterOffset) {
        // Northern hemisphere
        stringToUTF8(winterName, std_name, 17);
        stringToUTF8(summerName, dst_name, 17);
      } else {
        stringToUTF8(winterName, dst_name, 17);
        stringToUTF8(summerName, std_name, 17);
      }
    };

  var _emscripten_get_now = () => performance.now();
  
  var getHeapMax = () =>
      // Stay one Wasm page short of 4GB: while e.g. Chrome is able to allocate
      // full 4GB Wasm memories, the size will wrap back to 0 bytes in Wasm side
      // for any code that deals with heap sizes, which would require special
      // casing all heap size related code to treat 0 specially.
      2147483648;
  
  var alignMemory = (size, alignment) => {
      assert(alignment, "alignment argument is required");
      return Math.ceil(size / alignment) * alignment;
    };
  
  var growMemory = (size) => {
      var oldHeapSize = wasmMemory.buffer.byteLength;
      var pages = ((size - oldHeapSize + 65535) / 65536) | 0;
      try {
        // round size grow request up to wasm page size (fixed 64KB per spec)
        wasmMemory.grow(pages); // .grow() takes a delta compared to the previous size
        updateMemoryViews();
        return 1 /*success*/;
      } catch(e) {
        err(`growMemory: Attempted to grow heap from ${oldHeapSize} bytes to ${size} bytes, but got error: ${e}`);
      }
      // implicit 0 return to save code size (caller will cast "undefined" into 0
      // anyhow)
    };
  var _emscripten_resize_heap = (requestedSize) => {
      var oldSize = HEAPU8.length;
      // With CAN_ADDRESS_2GB or MEMORY64, pointers are already unsigned.
      requestedSize >>>= 0;
      // With multithreaded builds, races can happen (another thread might increase the size
      // in between), so return a failure, and let the caller retry.
      assert(requestedSize > oldSize);
  
      // Memory resize rules:
      // 1.  Always increase heap size to at least the requested size, rounded up
      //     to next page multiple.
      // 2a. If MEMORY_GROWTH_LINEAR_STEP == -1, excessively resize the heap
      //     geometrically: increase the heap size according to
      //     MEMORY_GROWTH_GEOMETRIC_STEP factor (default +20%), At most
      //     overreserve by MEMORY_GROWTH_GEOMETRIC_CAP bytes (default 96MB).
      // 2b. If MEMORY_GROWTH_LINEAR_STEP != -1, excessively resize the heap
      //     linearly: increase the heap size by at least
      //     MEMORY_GROWTH_LINEAR_STEP bytes.
      // 3.  Max size for the heap is capped at 2048MB-WASM_PAGE_SIZE, or by
      //     MAXIMUM_MEMORY, or by ASAN limit, depending on which is smallest
      // 4.  If we were unable to allocate as much memory, it may be due to
      //     over-eager decision to excessively reserve due to (3) above.
      //     Hence if an allocation fails, cut down on the amount of excess
      //     growth, in an attempt to succeed to perform a smaller allocation.
  
      // A limit is set for how much we can grow. We should not exceed that
      // (the wasm binary specifies it, so if we tried, we'd fail anyhow).
      var maxHeapSize = getHeapMax();
      if (requestedSize > maxHeapSize) {
        err(`Cannot enlarge memory, requested ${requestedSize} bytes, but the limit is ${maxHeapSize} bytes!`);
        return false;
      }
  
      // Loop through potential heap size increases. If we attempt a too eager
      // reservation that fails, cut down on the attempted size and reserve a
      // smaller bump instead. (max 3 times, chosen somewhat arbitrarily)
      for (var cutDown = 1; cutDown <= 4; cutDown *= 2) {
        var overGrownHeapSize = oldSize * (1 + 0.2 / cutDown); // ensure geometric growth
        // but limit overreserving (default to capping at +96MB overgrowth at most)
        overGrownHeapSize = Math.min(overGrownHeapSize, requestedSize + 100663296 );
  
        var newSize = Math.min(maxHeapSize, alignMemory(Math.max(requestedSize, overGrownHeapSize), 65536));
  
        var t0 = _emscripten_get_now();
        var replacement = growMemory(newSize);
        var t1 = _emscripten_get_now();
        dbg(`Heap resize call from ${oldSize} to ${newSize} took ${(t1 - t0)} msecs. Success: ${!!replacement}`);
        if (replacement) {
  
          return true;
        }
      }
      err(`Failed to grow the heap from ${oldSize} bytes to ${newSize} bytes, not enough memory!`);
      return false;
    };

  var ENV = {
  };
  
  var getExecutableName = () => thisProgram || './this.program';
  var getEnvStrings = () => {
      if (!getEnvStrings.strings) {
        // Default values.
        // Browser language detection #8751
        var lang = (globalThis.navigator?.language ?? 'C').replace('-', '_') + '.UTF-8';
        var env = {
          'USER': 'web_user',
          'LOGNAME': 'web_user',
          'PATH': '/',
          'PWD': '/',
          'HOME': '/home/web_user',
          'LANG': lang,
          '_': getExecutableName()
        };
        // Apply the user-provided values, if any.
        for (var x in ENV) {
          // x is a key in ENV; if ENV[x] is undefined, that means it was
          // explicitly set to be so. We allow user code to do that to
          // force variables with default values to remain unset.
          if (ENV[x] === undefined) delete env[x];
          else env[x] = ENV[x];
        }
        var strings = [];
        for (var x in env) {
          strings.push(`${x}=${env[x]}`);
        }
        getEnvStrings.strings = strings;
      }
      return getEnvStrings.strings;
    };
  
  var _environ_get = (__environ, environ_buf) => {
      var bufSize = 0;
      var envp = 0;
      for (var string of getEnvStrings()) {
        var ptr = environ_buf + bufSize;
        HEAPU32[(((__environ)+(envp))>>2)] = ptr;
        bufSize += stringToUTF8(string, ptr, Infinity) + 1;
        envp += 4;
      }
      return 0;
    };

  
  var _environ_sizes_get = (penviron_count, penviron_buf_size) => {
      var strings = getEnvStrings();
      HEAPU32[((penviron_count)>>2)] = strings.length;checkInt32(strings.length);
      var bufSize = 0;
      for (var string of strings) {
        bufSize += lengthBytesUTF8(string) + 1;
      }
      HEAPU32[((penviron_buf_size)>>2)] = bufSize;checkInt32(bufSize);
      return 0;
    };

  var SYSCALLS = {
  varargs:undefined,
  getStr(ptr) {
        var ret = UTF8ToString(ptr);
        return ret;
      },
  };
  var _fd_close = (fd) => {
      abort('fd_close called without SYSCALLS_REQUIRE_FILESYSTEM');
    };

  var _fd_read = (fd, iov, iovcnt, pnum) => {
      abort('fd_read called without SYSCALLS_REQUIRE_FILESYSTEM');
    };

  var INT53_MAX = 9007199254740992;
  
  var INT53_MIN = -9007199254740992;
  var bigintToI53Checked = (num) => (num < INT53_MIN || num > INT53_MAX) ? NaN : Number(num);
  function _fd_seek(fd, offset, whence, newOffset) {
    offset = bigintToI53Checked(offset);
  
  
      return 70;
    ;
  }

  var printCharBuffers = [null,[],[]];
  
  var printChar = (stream, curr) => {
      var buffer = printCharBuffers[stream];
      assert(buffer);
      if (curr === 0 || curr === 10) {
        (stream === 1 ? out : err)(UTF8ArrayToString(buffer));
        buffer.length = 0;
      } else {
        buffer.push(curr);
      }
    };
  
  var flush_NO_FILESYSTEM = () => {
      // flush anything remaining in the buffers during shutdown
      _fflush(0);
      if (printCharBuffers[1].length) printChar(1, 10);
      if (printCharBuffers[2].length) printChar(2, 10);
    };
  
  
  var _fd_write = (fd, iov, iovcnt, pnum) => {
      // hack to support printf in SYSCALLS_REQUIRE_FILESYSTEM=0
      var num = 0;
      for (var i = 0; i < iovcnt; i++) {
        var ptr = HEAPU32[((iov)>>2)];
        var len = HEAPU32[(((iov)+(4))>>2)];
        iov += 8;
        for (var j = 0; j < len; j++) {
          printChar(fd, HEAPU8[ptr+j]);
        }
        num += len;
      }
      HEAPU32[((pnum)>>2)] = num;checkInt32(num);
      return 0;
    };

  
  var runtimeKeepaliveCounter = 0;
  var keepRuntimeAlive = () => noExitRuntime || runtimeKeepaliveCounter > 0;
  var _proc_exit = (code) => {
      EXITSTATUS = code;
      if (!keepRuntimeAlive()) {
        Module['onExit']?.(code);
        ABORT = true;
      }
      quit_(code, new ExitStatus(code));
    };
  
  
  /** @param {boolean|number=} implicit */
  var exitJS = (status, implicit) => {
      EXITSTATUS = status;
  
      checkUnflushedContent();
  
      // if exit() was called explicitly, warn the user if the runtime isn't actually being shut down
      if (keepRuntimeAlive() && !implicit) {
        var msg = `program exited (with status: ${status}), but keepRuntimeAlive() is set (counter=${runtimeKeepaliveCounter}) due to an async operation, so halting execution but not exiting the runtime or preventing further async execution (you can use emscripten_force_exit, if you want to force a true shutdown)`;
        readyPromiseReject?.(msg);
        err(msg);
      }
  
      _proc_exit(status);
    };

  var handleException = (e) => {
      // Certain exception types we do not treat as errors since they are used for
      // internal control flow.
      // 1. ExitStatus, which is thrown by exit()
      // 2. "unwind", which is thrown by emscripten_unwind_to_js_event_loop() and others
      //    that wish to return to JS event loop.
      if (e instanceof ExitStatus || e == 'unwind') {
        return EXITSTATUS;
      }
      checkStackCookie();
      if (e instanceof WebAssembly.RuntimeError) {
        if (_emscripten_stack_get_current() <= 0) {
          err('Stack overflow detected.  You can try increasing -sSTACK_SIZE (currently set to 1048576)');
        }
      }
      quit_(1, e);
    };

  var getCFunc = (ident) => {
      var func = Module['_' + ident]; // closure exported function
      assert(func, 'Cannot call unknown function ' + ident + ', make sure it is exported');
      return func;
    };
  
  var writeArrayToMemory = (array, buffer) => {
      assert(array.length >= 0, 'writeArrayToMemory array must have a length (should be an array or typed array)')
      HEAP8.set(array, buffer);
    };
  
  
  
  var stackAlloc = (sz) => __emscripten_stack_alloc(sz);
  var stringToUTF8OnStack = (str) => {
      var size = lengthBytesUTF8(str) + 1;
      var ret = stackAlloc(size);
      stringToUTF8(str, ret, size);
      return ret;
    };
  
  
  
  
  
    /**
     * @param {string|null=} returnType
     * @param {Array=} argTypes
     * @param {Array=} args
     * @param {Object=} opts
     */
  var ccall = (ident, returnType, argTypes, args, opts) => {
      // For fast lookup of conversion functions
      var toC = {
        'string': (str) => {
          var ret = 0;
          if (str !== null && str !== undefined && str !== 0) { // null string
            ret = stringToUTF8OnStack(str);
          }
          return ret;
        },
        'array': (arr) => {
          var ret = stackAlloc(arr.length);
          writeArrayToMemory(arr, ret);
          return ret;
        }
      };
  
      function convertReturnValue(ret) {
        if (returnType === 'string') {
          return UTF8ToString(ret);
        }
        if (returnType === 'boolean') return Boolean(ret);
        return ret;
      }
  
      var func = getCFunc(ident);
      var cArgs = [];
      var stack = 0;
      assert(returnType !== 'array', 'Return type should not be "array".');
      if (args) {
        for (var i = 0; i < args.length; i++) {
          var converter = toC[argTypes[i]];
          if (converter) {
            if (stack === 0) stack = stackSave();
            cArgs[i] = converter(args[i]);
          } else {
            cArgs[i] = args[i];
          }
        }
      }
      var ret = func(...cArgs);
      function onDone(ret) {
        if (stack !== 0) stackRestore(stack);
        return convertReturnValue(ret);
      }
  
      ret = onDone(ret);
      return ret;
    };

  
    /**
     * @param {string=} returnType
     * @param {Array=} argTypes
     * @param {Object=} opts
     */
  var cwrap = (ident, returnType, argTypes, opts) => {
      return (...args) => ccall(ident, returnType, argTypes, args, opts);
    };




          Module._getCtx = function() {
              if (!Module._cachedCtx) {
                  Module._cachedCtx = Module.canvas.getContext('2d');
              }
              return Module._cachedCtx;
          };
          Module._getFontHeight = function(metrics, ctx) {
              if (metrics.fontBoundingBoxAscent !== undefined && metrics.fontBoundingBoxDescent !== undefined) {
                  return Math.ceil(metrics.fontBoundingBoxAscent + metrics.fontBoundingBoxDescent);
              }
              if (metrics.actualBoundingBoxAscent !== undefined && metrics.actualBoundingBoxDescent !== undefined) {
                  return Math.ceil(metrics.actualBoundingBoxAscent + metrics.actualBoundingBoxDescent);
              }
              var fontMatch = ctx.font.match(/(\d+)px/);
              return fontMatch ? parseInt(fontMatch[1]) : 12;
          };
          Module._CONSTANTS = {
              FONT_BOLD: 1, FONT_ITALIC: 2, FONT_MONOSPACE: 4,
              MIN_FONT_SIZE: 6, MAX_FONT_SIZE: 200, DEFAULT_FONT_SIZE: 12,
              TAP_DURATION_MS: 200, MAX_FILE_SIZE_BYTES: 50*1024*1024,
              MAX_IMAGE_CACHE_SIZE: 100, PLACEHOLDER_SIZE: 16,
              CACHE_EVICTION_PERCENTAGE: 0.25,
              PLACEHOLDER_BG_COLOR: '#f0f0f0', PLACEHOLDER_BORDER_COLOR: '#ccc',
              BASE64_CHUNK_SIZE: 32768, WHEEL_LINE_DELTA: 20, WHEEL_PAGE_DELTA: 400,
              RESIZE_DEBOUNCE_MS: 100
          };
          Module._validation = {
              clampFontSize: function(size) {
                  if (typeof size !== 'number' || isNaN(size) || !isFinite(size) || size <= 0) {
                      return Module._CONSTANTS.DEFAULT_FONT_SIZE;
                  }
                  return Math.max(Module._CONSTANTS.MIN_FONT_SIZE, Math.min(Module._CONSTANTS.MAX_FONT_SIZE, size));
              },
              isValidPointer: function(ptr) { return ptr !== 0 && ptr !== null && ptr !== undefined; },
              escapeHTML: function(text) {
                  if (!text) return '';
                  var div = document.createElement('div');
                  div.textContent = text;
                  return div.innerHTML;
              }
          };
          Module._getModifiers = function(event) {
              var modifiers = 0;
              if (event.ctrlKey) modifiers |= 1;
              if (event.shiftKey) modifiers |= 2;
              if (event.altKey) modifiers |= 4;
              if (event.metaKey) modifiers |= 8;
              return modifiers;
          };
          Module._initMouseEvents = function(canvas) {
              var getModifiers = Module._getModifiers;
              canvas.addEventListener('mousedown', function(e) { Module._WASM_Mouse(1, e.offsetX, e.offsetY, getModifiers(e)); });
              canvas.addEventListener('mouseup', function(e) { Module._WASM_Mouse(2, e.offsetX, e.offsetY, getModifiers(e)); });
              canvas.addEventListener('mousemove', function(e) { Module._WASM_Mouse(0, e.offsetX, e.offsetY, getModifiers(e)); });
          };
          Module._initWheelEvents = function(canvas) {
              canvas.addEventListener('wheel', function(e) {
                  e.preventDefault();
                  var delta = e.deltaY;
                  if (e.deltaMode === 1) delta *= 20;
                  if (e.deltaMode === 2) delta *= 400;
                  Module._WASM_Mouse(3, Math.round(delta), 0, Module._getModifiers(e));
              }, { passive: false });
          };
          Module._initTouchEvents = function(canvas) {
              var state = { lastX: 0, lastY: 0, startTime: 0, pinching: false, initDist: 0 };
              var getPos = function(t) { var r = canvas.getBoundingClientRect(); return { x: Math.round(t.clientX - r.left), y: Math.round(t.clientY - r.top) }; };
              var getDist = function(t1, t2) { return Math.sqrt(Math.pow(t1.clientX - t2.clientX, 2) + Math.pow(t1.clientY - t2.clientY, 2)); };
              canvas.addEventListener('touchstart', function(e) { e.preventDefault(); state.startTime = Date.now(); if (e.touches.length === 1) { var p = getPos(e.touches[0]); state.lastX = p.x; state.lastY = p.y; state.pinching = false; Module._WASM_Mouse(1, p.x, p.y, 0); } else if (e.touches.length === 2) { state.pinching = true; state.initDist = getDist(e.touches[0], e.touches[1]); } }, { passive: false });
              canvas.addEventListener('touchmove', function(e) { e.preventDefault(); if (e.touches.length === 1 && !state.pinching) { var p = getPos(e.touches[0]); Module._WASM_Mouse(0, p.x, p.y, 0); state.lastX = p.x; state.lastY = p.y; } else if (e.touches.length === 2) { var d = getDist(e.touches[0], e.touches[1]); Module._WASM_Mouse(3, Math.round((state.initDist - d) * 2), 0, 0); state.initDist = d; } }, { passive: false });
              canvas.addEventListener('touchend', function(e) { e.preventDefault(); if (e.touches.length === 0) { Module._WASM_Mouse(2, state.lastX, state.lastY, 0); state.pinching = false; } }, { passive: false });
          };
          Module._initKeyboardEvents = function() {
              var shortcuts = ['s','o','n','w','z','y','x','c','v','a','f'];
              window.addEventListener('keydown', function(e) { if ((e.ctrlKey || e.metaKey) && shortcuts.indexOf(e.key.toLowerCase()) !== -1) e.preventDefault(); Module._WASM_Key(0, e.keyCode, Module._getModifiers(e)); });
              window.addEventListener('keyup', function(e) { Module._WASM_Key(1, e.keyCode, Module._getModifiers(e)); });
          };
          Module._initResizeHandler = function(canvas) {
              var onResize = function() { var w = canvas.clientWidth, h = canvas.clientHeight; if (canvas.width !== w || canvas.height !== h) { canvas.width = w; canvas.height = h; Module._WASM_Resize(w, h); } };
              window.addEventListener('resize', function() { setTimeout(onResize, 100); });
              onResize();
          };
          Module._colorUtils = {
              decode: function(color) {
                  return { r: (color >> 16) & 0xFF, g: (color >> 8) & 0xFF, b: color & 0xFF };
              },
              toRGBString: function(color) {
                  var c = this.decode(color);
                  return 'rgb(' + c.r + ',' + c.g + ',' + c.b + ')';
              },
              fromHex: function(hexString) {
                  if (!hexString || typeof hexString !== 'string' || !hexString.match(/^#[0-9A-Fa-f]{6}$/)) {
                      return 0x000000;
                  }
                  var hex = hexString.substring(1);
                  return (parseInt(hex.substring(0, 2), 16) << 16) | (parseInt(hex.substring(2, 4), 16) << 8) | parseInt(hex.substring(4, 6), 16);
              },
              toHex: function(color) {
                  var c = this.decode(color);
                  return '#' + ('0' + c.r.toString(16)).slice(-2) + ('0' + c.g.toString(16)).slice(-2) + ('0' + c.b.toString(16)).slice(-2);
              }
          };
          Module._penStyles = {
              0: { strokeStyle: 'rgb(200,200,200)', lineWidth: 1, lineDash: [] },
              1: { strokeStyle: 'rgb(230,230,230)', lineWidth: 1, lineDash: [] },
              2: { strokeStyle: 'rgb(0,120,215)', lineWidth: 1, lineDash: [] },
              3: { strokeStyle: 'rgb(100,100,100)', lineWidth: 1, lineDash: [] },
              4: { strokeStyle: 'rgb(255,0,0)', lineWidth: 1, lineDash: [] },
              5: { strokeStyle: 'rgb(211,211,211)', lineWidth: 1, lineDash: [] },
              6: { strokeStyle: 'rgb(0,0,0)', lineWidth: 1, lineDash: [] },
              7: { strokeStyle: 'rgb(255,255,255)', lineWidth: 1, lineDash: [] },
              8: { strokeStyle: 'rgb(128,128,128)', lineWidth: 1, lineDash: [] }
          };
          Module._brushStyles = {
              0: 'transparent',
              1: 'rgb(255,255,255)',
              2: 'rgb(0,0,0)',
              3: 'rgb(211,211,211)'
          };
          // Text foreground color stored separately from brush color
          // This fixes the issue where SetBrushColor overwrites text color
          // since both used ctx.fillStyle in canvas 2D API
          Module._textForegroundColor = 'rgb(0,0,0)';
      ;
// End JS library code

// include: postlibrary.js
// This file is included after the automatically-generated JS library code
// but before the wasm module is created.

{

  // Begin ATMODULES hooks
  if (Module['noExitRuntime']) noExitRuntime = Module['noExitRuntime'];
if (Module['print']) out = Module['print'];
if (Module['printErr']) err = Module['printErr'];
if (Module['wasmBinary']) wasmBinary = Module['wasmBinary'];

Module['FS_createDataFile'] = FS.createDataFile;
Module['FS_createPreloadedFile'] = FS.createPreloadedFile;

  // End ATMODULES hooks

  checkIncomingModuleAPI();

  if (Module['arguments']) arguments_ = Module['arguments'];
  if (Module['thisProgram']) thisProgram = Module['thisProgram'];

  // Assertions on removed incoming Module JS APIs.
  assert(typeof Module['memoryInitializerPrefixURL'] == 'undefined', 'Module.memoryInitializerPrefixURL option was removed, use Module.locateFile instead');
  assert(typeof Module['pthreadMainPrefixURL'] == 'undefined', 'Module.pthreadMainPrefixURL option was removed, use Module.locateFile instead');
  assert(typeof Module['cdInitializerPrefixURL'] == 'undefined', 'Module.cdInitializerPrefixURL option was removed, use Module.locateFile instead');
  assert(typeof Module['filePackagePrefixURL'] == 'undefined', 'Module.filePackagePrefixURL option was removed, use Module.locateFile instead');
  assert(typeof Module['read'] == 'undefined', 'Module.read option was removed');
  assert(typeof Module['readAsync'] == 'undefined', 'Module.readAsync option was removed (modify readAsync in JS)');
  assert(typeof Module['readBinary'] == 'undefined', 'Module.readBinary option was removed (modify readBinary in JS)');
  assert(typeof Module['setWindowTitle'] == 'undefined', 'Module.setWindowTitle option was removed (modify emscripten_set_window_title in JS)');
  assert(typeof Module['TOTAL_MEMORY'] == 'undefined', 'Module.TOTAL_MEMORY has been renamed Module.INITIAL_MEMORY');
  assert(typeof Module['ENVIRONMENT'] == 'undefined', 'Module.ENVIRONMENT has been deprecated. To force the environment, use the ENVIRONMENT compile-time option (for example, -sENVIRONMENT=web or -sENVIRONMENT=node)');
  assert(typeof Module['STACK_SIZE'] == 'undefined', 'STACK_SIZE can no longer be set at runtime.  Use -sSTACK_SIZE at link time')
  // If memory is defined in wasm, the user can't provide it, or set INITIAL_MEMORY
  assert(typeof Module['wasmMemory'] == 'undefined', 'Use of `wasmMemory` detected.  Use -sIMPORTED_MEMORY to define wasmMemory externally');
  assert(typeof Module['INITIAL_MEMORY'] == 'undefined', 'Detected runtime INITIAL_MEMORY setting.  Use -sIMPORTED_MEMORY to define wasmMemory dynamically');

  if (Module['preInit']) {
    if (typeof Module['preInit'] == 'function') Module['preInit'] = [Module['preInit']];
    while (Module['preInit'].length > 0) {
      Module['preInit'].shift()();
    }
  }
  consumedModuleProp('preInit');
}

// Begin runtime exports
  Module['ccall'] = ccall;
  Module['cwrap'] = cwrap;
  Module['UTF8ToString'] = UTF8ToString;
  Module['stringToUTF8'] = stringToUTF8;
  Module['lengthBytesUTF8'] = lengthBytesUTF8;
  var missingLibrarySymbols = [
  'writeI53ToI64',
  'writeI53ToI64Clamped',
  'writeI53ToI64Signaling',
  'writeI53ToU64Clamped',
  'writeI53ToU64Signaling',
  'readI53FromI64',
  'readI53FromU64',
  'convertI32PairToI53',
  'convertI32PairToI53Checked',
  'convertU32PairToI53',
  'getTempRet0',
  'setTempRet0',
  'createNamedFunction',
  'zeroMemory',
  'withStackSave',
  'strError',
  'inetPton4',
  'inetNtop4',
  'inetPton6',
  'inetNtop6',
  'readSockaddr',
  'writeSockaddr',
  'readEmAsmArgs',
  'jstoi_q',
  'autoResumeAudioContext',
  'getDynCaller',
  'dynCall',
  'runtimeKeepalivePush',
  'runtimeKeepalivePop',
  'callUserCallback',
  'maybeExit',
  'asyncLoad',
  'asmjsMangle',
  'mmapAlloc',
  'HandleAllocator',
  'getUniqueRunDependency',
  'addRunDependency',
  'removeRunDependency',
  'addOnInit',
  'addOnPostCtor',
  'addOnPreMain',
  'addOnExit',
  'STACK_SIZE',
  'STACK_ALIGN',
  'POINTER_SIZE',
  'ASSERTIONS',
  'convertJsFunctionToWasm',
  'getEmptyTableSlot',
  'updateTableMap',
  'getFunctionAddress',
  'addFunction',
  'removeFunction',
  'intArrayFromString',
  'intArrayToString',
  'AsciiToString',
  'stringToAscii',
  'UTF16ToString',
  'stringToUTF16',
  'lengthBytesUTF16',
  'UTF32ToString',
  'stringToUTF32',
  'lengthBytesUTF32',
  'stringToNewUTF8',
  'registerKeyEventCallback',
  'maybeCStringToJsString',
  'findEventTarget',
  'getBoundingClientRect',
  'fillMouseEventData',
  'registerMouseEventCallback',
  'registerWheelEventCallback',
  'registerUiEventCallback',
  'registerFocusEventCallback',
  'fillDeviceOrientationEventData',
  'registerDeviceOrientationEventCallback',
  'fillDeviceMotionEventData',
  'registerDeviceMotionEventCallback',
  'screenOrientation',
  'fillOrientationChangeEventData',
  'registerOrientationChangeEventCallback',
  'fillFullscreenChangeEventData',
  'registerFullscreenChangeEventCallback',
  'JSEvents_requestFullscreen',
  'JSEvents_resizeCanvasForFullscreen',
  'registerRestoreOldStyle',
  'hideEverythingExceptGivenElement',
  'restoreHiddenElements',
  'setLetterbox',
  'softFullscreenResizeWebGLRenderTarget',
  'doRequestFullscreen',
  'fillPointerlockChangeEventData',
  'registerPointerlockChangeEventCallback',
  'registerPointerlockErrorEventCallback',
  'requestPointerLock',
  'fillVisibilityChangeEventData',
  'registerVisibilityChangeEventCallback',
  'registerTouchEventCallback',
  'fillGamepadEventData',
  'registerGamepadEventCallback',
  'registerBeforeUnloadEventCallback',
  'fillBatteryEventData',
  'registerBatteryEventCallback',
  'setCanvasElementSize',
  'getCanvasElementSize',
  'jsStackTrace',
  'getCallstack',
  'convertPCtoSourceLocation',
  'checkWasiClock',
  'wasiRightsToMuslOFlags',
  'wasiOFlagsToMuslOFlags',
  'initRandomFill',
  'randomFill',
  'safeSetTimeout',
  'setImmediateWrapped',
  'safeRequestAnimationFrame',
  'clearImmediateWrapped',
  'registerPostMainLoop',
  'registerPreMainLoop',
  'getPromise',
  'makePromise',
  'idsToPromises',
  'makePromiseCallback',
  'findMatchingCatch',
  'Browser_asyncPrepareDataCounter',
  'isLeapYear',
  'ydayFromDate',
  'arraySum',
  'addDays',
  'getSocketFromFD',
  'getSocketAddress',
  'heapObjectForWebGLType',
  'toTypedArrayIndex',
  'webgl_enable_ANGLE_instanced_arrays',
  'webgl_enable_OES_vertex_array_object',
  'webgl_enable_WEBGL_draw_buffers',
  'webgl_enable_WEBGL_multi_draw',
  'webgl_enable_EXT_polygon_offset_clamp',
  'webgl_enable_EXT_clip_control',
  'webgl_enable_WEBGL_polygon_mode',
  'emscriptenWebGLGet',
  'computeUnpackAlignedImageSize',
  'colorChannelsInGlTextureFormat',
  'emscriptenWebGLGetTexPixelData',
  'emscriptenWebGLGetUniform',
  'webglGetUniformLocation',
  'webglPrepareUniformLocationsBeforeFirstUse',
  'webglGetLeftBracePos',
  'emscriptenWebGLGetVertexAttrib',
  '__glGetActiveAttribOrUniform',
  'writeGLArray',
  'registerWebGlEventCallback',
  'runAndAbortIfError',
  'ALLOC_NORMAL',
  'ALLOC_STACK',
  'allocate',
  'writeStringToMemory',
  'writeAsciiToMemory',
  'allocateUTF8',
  'allocateUTF8OnStack',
  'demangle',
  'stackTrace',
  'getNativeTypeSize',
];
missingLibrarySymbols.forEach(missingLibrarySymbol)

  var unexportedSymbols = [
  'run',
  'out',
  'err',
  'callMain',
  'abort',
  'wasmExports',
  'HEAPF32',
  'HEAPF64',
  'HEAP8',
  'HEAPU8',
  'HEAP16',
  'HEAPU16',
  'HEAP32',
  'HEAPU32',
  'HEAP64',
  'HEAPU64',
  'writeStackCookie',
  'checkStackCookie',
  'INT53_MAX',
  'INT53_MIN',
  'bigintToI53Checked',
  'stackSave',
  'stackRestore',
  'stackAlloc',
  'ptrToString',
  'exitJS',
  'getHeapMax',
  'growMemory',
  'ENV',
  'setStackLimits',
  'ERRNO_CODES',
  'DNS',
  'Protocols',
  'Sockets',
  'timers',
  'warnOnce',
  'readEmAsmArgsArray',
  'getExecutableName',
  'handleException',
  'keepRuntimeAlive',
  'alignMemory',
  'wasmTable',
  'wasmMemory',
  'noExitRuntime',
  'addOnPreRun',
  'addOnPostRun',
  'freeTableIndexes',
  'functionsInTableMap',
  'setValue',
  'getValue',
  'PATH',
  'PATH_FS',
  'UTF8Decoder',
  'UTF8ArrayToString',
  'stringToUTF8Array',
  'UTF16Decoder',
  'stringToUTF8OnStack',
  'writeArrayToMemory',
  'JSEvents',
  'specialHTMLTargets',
  'findCanvasEventTarget',
  'currentFullscreenStrategy',
  'restoreOldWindowedStyle',
  'UNWIND_CACHE',
  'ExitStatus',
  'getEnvStrings',
  'flush_NO_FILESYSTEM',
  'emSetImmediate',
  'emClearImmediate_deps',
  'emClearImmediate',
  'promiseMap',
  'uncaughtExceptionCount',
  'exceptionLast',
  'exceptionCaught',
  'ExceptionInfo',
  'Browser',
  'requestFullscreen',
  'requestFullScreen',
  'setCanvasSize',
  'getUserMedia',
  'createContext',
  'getPreloadedImageData__data',
  'wget',
  'MONTH_DAYS_REGULAR',
  'MONTH_DAYS_LEAP',
  'MONTH_DAYS_REGULAR_CUMULATIVE',
  'MONTH_DAYS_LEAP_CUMULATIVE',
  'SYSCALLS',
  'tempFixedLengthArray',
  'miniTempWebGLFloatBuffers',
  'miniTempWebGLIntBuffers',
  'GL',
  'AL',
  'GLUT',
  'EGL',
  'GLEW',
  'IDBStore',
  'SDL',
  'SDL_gfx',
  'print',
  'printErr',
  'jstoi_s',
  '__tsHelpers',
];
unexportedSymbols.forEach(unexportedRuntimeSymbol);

  // End runtime exports
  // Begin JS library exports
  // End JS library exports

// end include: postlibrary.js

function checkIncomingModuleAPI() {
  ignoredModuleProp('fetchSettings');
}

// Imports from the Wasm binary.
var _WASM_FileLoaded = Module['_WASM_FileLoaded'] = makeInvalidEarlyAccess('_WASM_FileLoaded');
var _WASM_Mouse = Module['_WASM_Mouse'] = makeInvalidEarlyAccess('_WASM_Mouse');
var _WASM_Key = Module['_WASM_Key'] = makeInvalidEarlyAccess('_WASM_Key');
var _WASM_Resize = Module['_WASM_Resize'] = makeInvalidEarlyAccess('_WASM_Resize');
var _WASM_Action = Module['_WASM_Action'] = makeInvalidEarlyAccess('_WASM_Action');
var _main = Module['_main'] = makeInvalidEarlyAccess('_main');
var _fflush = makeInvalidEarlyAccess('_fflush');
var _emscripten_stack_get_end = makeInvalidEarlyAccess('_emscripten_stack_get_end');
var _emscripten_stack_get_base = makeInvalidEarlyAccess('_emscripten_stack_get_base');
var _emscripten_stack_init = makeInvalidEarlyAccess('_emscripten_stack_init');
var _emscripten_stack_get_free = makeInvalidEarlyAccess('_emscripten_stack_get_free');
var __emscripten_stack_restore = makeInvalidEarlyAccess('__emscripten_stack_restore');
var __emscripten_stack_alloc = makeInvalidEarlyAccess('__emscripten_stack_alloc');
var _emscripten_stack_get_current = makeInvalidEarlyAccess('_emscripten_stack_get_current');
var ___set_stack_limits = Module['___set_stack_limits'] = makeInvalidEarlyAccess('___set_stack_limits');
var memory = makeInvalidEarlyAccess('memory');
var __indirect_function_table = makeInvalidEarlyAccess('__indirect_function_table');
var wasmMemory = makeInvalidEarlyAccess('wasmMemory');

function assignWasmExports(wasmExports) {
  assert(typeof wasmExports['WASM_FileLoaded'] != 'undefined', 'missing Wasm export: WASM_FileLoaded');
  assert(typeof wasmExports['WASM_Mouse'] != 'undefined', 'missing Wasm export: WASM_Mouse');
  assert(typeof wasmExports['WASM_Key'] != 'undefined', 'missing Wasm export: WASM_Key');
  assert(typeof wasmExports['WASM_Resize'] != 'undefined', 'missing Wasm export: WASM_Resize');
  assert(typeof wasmExports['WASM_Action'] != 'undefined', 'missing Wasm export: WASM_Action');
  assert(typeof wasmExports['main'] != 'undefined', 'missing Wasm export: main');
  assert(typeof wasmExports['fflush'] != 'undefined', 'missing Wasm export: fflush');
  assert(typeof wasmExports['emscripten_stack_get_end'] != 'undefined', 'missing Wasm export: emscripten_stack_get_end');
  assert(typeof wasmExports['emscripten_stack_get_base'] != 'undefined', 'missing Wasm export: emscripten_stack_get_base');
  assert(typeof wasmExports['emscripten_stack_init'] != 'undefined', 'missing Wasm export: emscripten_stack_init');
  assert(typeof wasmExports['emscripten_stack_get_free'] != 'undefined', 'missing Wasm export: emscripten_stack_get_free');
  assert(typeof wasmExports['_emscripten_stack_restore'] != 'undefined', 'missing Wasm export: _emscripten_stack_restore');
  assert(typeof wasmExports['_emscripten_stack_alloc'] != 'undefined', 'missing Wasm export: _emscripten_stack_alloc');
  assert(typeof wasmExports['emscripten_stack_get_current'] != 'undefined', 'missing Wasm export: emscripten_stack_get_current');
  assert(typeof wasmExports['__set_stack_limits'] != 'undefined', 'missing Wasm export: __set_stack_limits');
  assert(typeof wasmExports['memory'] != 'undefined', 'missing Wasm export: memory');
  assert(typeof wasmExports['__indirect_function_table'] != 'undefined', 'missing Wasm export: __indirect_function_table');
  _WASM_FileLoaded = Module['_WASM_FileLoaded'] = createExportWrapper('WASM_FileLoaded', 3);
  _WASM_Mouse = Module['_WASM_Mouse'] = createExportWrapper('WASM_Mouse', 4);
  _WASM_Key = Module['_WASM_Key'] = createExportWrapper('WASM_Key', 3);
  _WASM_Resize = Module['_WASM_Resize'] = createExportWrapper('WASM_Resize', 2);
  _WASM_Action = Module['_WASM_Action'] = createExportWrapper('WASM_Action', 1);
  _main = Module['_main'] = createExportWrapper('main', 2);
  _fflush = createExportWrapper('fflush', 1);
  _emscripten_stack_get_end = wasmExports['emscripten_stack_get_end'];
  _emscripten_stack_get_base = wasmExports['emscripten_stack_get_base'];
  _emscripten_stack_init = wasmExports['emscripten_stack_init'];
  _emscripten_stack_get_free = wasmExports['emscripten_stack_get_free'];
  __emscripten_stack_restore = wasmExports['_emscripten_stack_restore'];
  __emscripten_stack_alloc = wasmExports['_emscripten_stack_alloc'];
  _emscripten_stack_get_current = wasmExports['emscripten_stack_get_current'];
  ___set_stack_limits = Module['___set_stack_limits'] = createExportWrapper('__set_stack_limits', 2);
  memory = wasmMemory = wasmExports['memory'];
  __indirect_function_table = wasmExports['__indirect_function_table'];
}

var wasmImports = {
  /** @export */
  JS_AskNumber: _JS_AskNumber,
  /** @export */
  JS_AskText: _JS_AskText,
  /** @export */
  JS_DownloadFile: _JS_DownloadFile,
  /** @export */
  JS_DrawBitmap: _JS_DrawBitmap,
  /** @export */
  JS_DrawLine: _JS_DrawLine,
  /** @export */
  JS_DrawRectangle: _JS_DrawRectangle,
  /** @export */
  JS_DrawRoundedRectangle: _JS_DrawRoundedRectangle,
  /** @export */
  JS_DrawText: _JS_DrawText,
  /** @export */
  JS_GetCharHeight: _JS_GetCharHeight,
  /** @export */
  JS_GetClipboardText: _JS_GetClipboardText,
  /** @export */
  JS_GetLastFileSize: _JS_GetLastFileSize,
  /** @export */
  JS_GetSelectedFont: _JS_GetSelectedFont,
  /** @export */
  JS_GetSelectedFontSize: _JS_GetSelectedFontSize,
  /** @export */
  JS_GetTextWidth: _JS_GetTextWidth,
  /** @export */
  JS_InitInput: _JS_InitInput,
  /** @export */
  JS_IsDarkMode: _JS_IsDarkMode,
  /** @export */
  JS_LaunchBrowser: _JS_LaunchBrowser,
  /** @export */
  JS_MenuBar_Append: _JS_MenuBar_Append,
  /** @export */
  JS_Menu_Append: _JS_Menu_Append,
  /** @export */
  JS_Menu_AppendSubMenu: _JS_Menu_AppendSubMenu,
  /** @export */
  JS_Menu_Check: _JS_Menu_Check,
  /** @export */
  JS_Menu_Create: _JS_Menu_Create,
  /** @export */
  JS_PickColor: _JS_PickColor,
  /** @export */
  JS_ReadFile: _JS_ReadFile,
  /** @export */
  JS_SelectFont: _JS_SelectFont,
  /** @export */
  JS_SetBrush: _JS_SetBrush,
  /** @export */
  JS_SetBrushColor: _JS_SetBrushColor,
  /** @export */
  JS_SetClipboardText: _JS_SetClipboardText,
  /** @export */
  JS_SetFont: _JS_SetFont,
  /** @export */
  JS_SetPen: _JS_SetPen,
  /** @export */
  JS_SetPenColor: _JS_SetPenColor,
  /** @export */
  JS_SetTextBackground: _JS_SetTextBackground,
  /** @export */
  JS_SetTextForeground: _JS_SetTextForeground,
  /** @export */
  JS_ShowMessage: _JS_ShowMessage,
  /** @export */
  JS_SingleChoice: _JS_SingleChoice,
  /** @export */
  JS_Toolbar_AddDropdown: _JS_Toolbar_AddDropdown,
  /** @export */
  JS_Toolbar_AddInput: _JS_Toolbar_AddInput,
  /** @export */
  JS_Toolbar_AddLabel: _JS_Toolbar_AddLabel,
  /** @export */
  JS_Toolbar_AddSeparator: _JS_Toolbar_AddSeparator,
  /** @export */
  JS_Toolbar_AddTool: _JS_Toolbar_AddTool,
  /** @export */
  JS_Toolbar_Create: _JS_Toolbar_Create,
  /** @export */
  JS_TriggerUpload: _JS_TriggerUpload,
  /** @export */
  __assert_fail: ___assert_fail,
  /** @export */
  __cxa_throw: ___cxa_throw,
  /** @export */
  __handle_stack_overflow: ___handle_stack_overflow,
  /** @export */
  _abort_js: __abort_js,
  /** @export */
  _tzset_js: __tzset_js,
  /** @export */
  emscripten_resize_heap: _emscripten_resize_heap,
  /** @export */
  environ_get: _environ_get,
  /** @export */
  environ_sizes_get: _environ_sizes_get,
  /** @export */
  fd_close: _fd_close,
  /** @export */
  fd_read: _fd_read,
  /** @export */
  fd_seek: _fd_seek,
  /** @export */
  fd_write: _fd_write
};


// include: postamble.js
// === Auto-generated postamble setup entry stuff ===

var calledRun;

function callMain() {
  assert(typeof onPreRuns === 'undefined' || onPreRuns.length == 0, 'cannot call main when preRun functions remain to be called');

  var entryFunction = _main;

  var argc = 0;
  var argv = 0;

  try {

    var ret = entryFunction(argc, argv);

    // if we're not running an evented main loop, it's time to exit
    exitJS(ret, /* implicit = */ true);
    return ret;
  } catch (e) {
    return handleException(e);
  }
}

function stackCheckInit() {
  // This is normally called automatically during __wasm_call_ctors but need to
  // get these values before even running any of the ctors so we call it redundantly
  // here.
  _emscripten_stack_init();
  // TODO(sbc): Move writeStackCookie to native to to avoid this.
  writeStackCookie();
}

function run() {

  stackCheckInit();

  preRun();

  function doRun() {
    // run may have just been called through dependencies being fulfilled just in this very frame,
    // or while the async setStatus time below was happening
    assert(!calledRun);
    calledRun = true;
    Module['calledRun'] = true;

    if (ABORT) return;

    initRuntime();

    preMain();

    readyPromiseResolve?.(Module);
    Module['onRuntimeInitialized']?.();
    consumedModuleProp('onRuntimeInitialized');

    var noInitialRun = Module['noInitialRun'] || false;
    if (!noInitialRun) callMain();

    postRun();
  }

  if (Module['setStatus']) {
    Module['setStatus']('Running...');
    setTimeout(() => {
      setTimeout(() => Module['setStatus'](''), 1);
      doRun();
    }, 1);
  } else
  {
    doRun();
  }
  checkStackCookie();
}

function checkUnflushedContent() {
  // Compiler settings do not allow exiting the runtime, so flushing
  // the streams is not possible. but in ASSERTIONS mode we check
  // if there was something to flush, and if so tell the user they
  // should request that the runtime be exitable.
  // Normally we would not even include flush() at all, but in ASSERTIONS
  // builds we do so just for this check, and here we see if there is any
  // content to flush, that is, we check if there would have been
  // something a non-ASSERTIONS build would have not seen.
  // How we flush the streams depends on whether we are in SYSCALLS_REQUIRE_FILESYSTEM=0
  // mode (which has its own special function for this; otherwise, all
  // the code is inside libc)
  var oldOut = out;
  var oldErr = err;
  var has = false;
  out = err = (x) => {
    has = true;
  }
  try { // it doesn't matter if it fails
    flush_NO_FILESYSTEM();
  } catch(e) {}
  out = oldOut;
  err = oldErr;
  if (has) {
    warnOnce('stdio streams had content in them that was not flushed. you should set EXIT_RUNTIME to 1 (see the Emscripten FAQ), or make sure to emit a newline when you printf etc.');
    warnOnce('(this may also be due to not including full filesystem support - try building with -sFORCE_FILESYSTEM)');
  }
}

var wasmExports;

// In modularize mode the generated code is within a factory function so we
// can use await here (since it's not top-level-await).
wasmExports = await (createWasm());

run();

// end include: postamble.js

// include: postamble_modularize.js
// In MODULARIZE mode we wrap the generated code in a factory function
// and return either the Module itself, or a promise of the module.
//
// We assign to the `moduleRtn` global here and configure closure to see
// this as and extern so it won't get minified.

if (runtimeInitialized)  {
  moduleRtn = Module;
} else {
  // Set up the promise that indicates the Module is initialized
  moduleRtn = new Promise((resolve, reject) => {
    readyPromiseResolve = resolve;
    readyPromiseReject = reject;
  });
}

// Assertion for attempting to access module properties on the incoming
// moduleArg.  In the past we used this object as the prototype of the module
// and assigned properties to it, but now we return a distinct object.  This
// keeps the instance private until it is ready (i.e the promise has been
// resolved).
for (const prop of Object.keys(Module)) {
  if (!(prop in moduleArg)) {
    Object.defineProperty(moduleArg, prop, {
      configurable: true,
      get() {
        abort(`Access to module property ('${prop}') is no longer possible via the module constructor argument; Instead, use the result of the module constructor.`)
      }
    });
  }
}
// end include: postamble_modularize.js



    return moduleRtn;
  };
})();

// Export using a UMD style export, or ES6 exports if selected
if (typeof exports === 'object' && typeof module === 'object') {
  module.exports = TreeSheets;
  // This default export looks redundant, but it allows TS to import this
  // commonjs style module.
  module.exports.default = TreeSheets;
} else if (typeof define === 'function' && define['amd'])
  define([], () => TreeSheets);

