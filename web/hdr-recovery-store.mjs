// Temporary, origin-local transfer across GPU recovery page reloads.
export async function openHdrRecoveryStore(indexedDB,{now=Date.now,databaseName='voxy-hdr-recovery'}={}) {
  const db=await new Promise((resolve,reject)=>{
    const request=indexedDB.open(databaseName,1);
    request.onupgradeneeded=()=>request.result.createObjectStore('sources');
    request.onsuccess=()=>resolve(request.result);
    request.onerror=()=>reject(request.error);
  });
  async function transaction(mode,operation) {
    return new Promise((resolve,reject)=>{
      const tx=db.transaction('sources',mode);
      const request=operation(tx.objectStore('sources'));
      tx.oncomplete=()=>resolve(request.result);
      tx.onabort=()=>reject(tx.error ?? new Error('HDR recovery transaction aborted'));
      tx.onerror=()=>reject(tx.error);
    });
  }
  function valid(value) {
    return value && Number.isFinite(value.created) && value.created<=now()
      && now()-value.created<=60*60*1000
      && value.bytes instanceof Uint8Array && value.bytes.length>0 && value.bytes.length<=32*1024*1024;
  }
  // Reclaim abandoned transfers when recovery storage is next opened.
  try {
    await new Promise((resolve,reject)=>{
      const tx=db.transaction('sources','readwrite');
      const request=tx.objectStore('sources').openCursor();
      request.onsuccess=()=>{
        const cursor=request.result;
        if(!cursor) return;
        if(!valid(cursor.value)) cursor.delete();
        cursor.continue();
      };
      tx.oncomplete=resolve;
      tx.onabort=()=>reject(tx.error ?? new Error('HDR recovery cleanup aborted'));
      tx.onerror=()=>reject(tx.error);
    });
  } catch(error) {db.close();throw error;}
  return {
    async save(id,snapshot) {
      if(!snapshot || !(snapshot.bytes instanceof Uint8Array) || !snapshot.bytes.length || snapshot.bytes.length>32*1024*1024)
        throw new Error('Invalid HDR recovery source');
      await transaction('readwrite',store=>store.put({...snapshot,created:now()},id));
    },
    async read(id) {
      const value=await transaction('readonly',store=>store.get(id));
      if(!valid(value)) {
        if(value) await transaction('readwrite',store=>store.delete(id));
        return null;
      }
      return {name:String(value.name),bytes:value.bytes};
    },
    async remove(id){await transaction('readwrite',store=>store.delete(id));},
    close(){db.close();},
  };
}
