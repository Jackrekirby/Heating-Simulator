console.log('WebAssembly');

var importObject = {
    imports: { imported_func: arg => console.log(arg) }
};

// WebAssembly.instantiateStreaming(fetch('simple.wasm'), importObject)
//     .then(obj => obj.instance.exports.exported_func());

fetch('simple.wasm').then(response =>
    response.arrayBuffer()
).then(bytes =>
    WebAssembly.instantiate(bytes, importObject)
).then(results => {
    results.instance.exports.exported_func();
});