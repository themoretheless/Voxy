// Serialize file decoding/import while the WASM engine holds an async mutable borrow.
export function createHdrLoader({load, busyChanged, result, maxBytes=32*1024*1024}) {
  let busy=false;
  let retained=null;
  async function apply(bytes,name,importer) {
    busy=true;busyChanged(true);
    try {
      // Keep recovery bytes separate from WASM ownership and caller mutation.
      const snapshot=bytes.slice();
      await importer(bytes);
      retained={name,bytes:snapshot};
      result(true,name);return true;
    } catch(error) {result(false,String(error));return false;}
    finally {busy=false;busyChanged(false);}
  }
  return {
    get busy(){return busy;},
    get hasEnvironment(){return retained!==null;},
    snapshot(){return retained ? {name:retained.name,bytes:retained.bytes.slice()} : null;},
    // Rebind to a recreated engine without reading the original File again.
    async restore(importer=load) {
      if(busy || !retained) return false;
      return apply(retained.bytes.slice(),retained.name,importer);
    },
    async run(file) {
      if(busy || !file) return false;
      if(file.size===0 || file.size>maxBytes) {result(false,'HDR file must be between 1 byte and 32 MiB');return false;}
      busy=true;busyChanged(true);
      try {
        const bytes=new Uint8Array(await file.arrayBuffer());
        if(bytes.length===0 || bytes.length>maxBytes) throw new Error('HDR file exceeds source limit');
        const snapshot=bytes.slice();
        await load(bytes);
        retained={name:file.name,bytes:snapshot};
        result(true,file.name);return true;
      } catch(error) {result(false,String(error));return false;}
      finally {busy=false;busyChanged(false);}
    },
  };
}
