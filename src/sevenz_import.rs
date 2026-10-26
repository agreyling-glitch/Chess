//! Worker-only streaming 7z decoding. Input and output are bounded JS callbacks.
#[cfg(target_arch="wasm32")]
mod browser {
    use std::io::{Read,Seek,SeekFrom};
    use wasm_bindgen::prelude::*;
    const MAX:u64=64*1024*1024*1024;
    struct Input {size:u64,position:u64,read:js_sys::Function}
    impl Read for Input {
        fn read(&mut self,buf:&mut[u8])->std::io::Result<usize>{
            let length=(buf.len() as u64).min(self.size.saturating_sub(self.position)).min(1024*1024) as usize;
            if length==0{return Ok(0);}
            let value=self.read.call2(&JsValue::NULL,&JsValue::from_f64(self.position as f64),&JsValue::from_f64(length as f64)).map_err(|_|std::io::Error::other("Could not read local 7z file"))?;
            let bytes=js_sys::Uint8Array::new(&value);if bytes.length() as usize!=length{return Err(std::io::Error::other("Truncated local 7z file"));}bytes.copy_to(&mut buf[..length]);self.position+=length as u64;Ok(length)
        }
    }
    impl Seek for Input {
        fn seek(&mut self,seek:SeekFrom)->std::io::Result<u64>{let position=match seek{SeekFrom::Start(n)=>i128::from(n),SeekFrom::Current(n)=>i128::from(self.position)+i128::from(n),SeekFrom::End(n)=>i128::from(self.size)+i128::from(n)};if position<0||position>i128::from(self.size){return Err(std::io::Error::other("Invalid 7z offset"));}self.position=position as u64;Ok(self.position)}
    }
    type Reader=sevenz_rust2::ArchiveReader<std::io::BufReader<Input>>;
    fn reader(size:f64,read:js_sys::Function)->Result<Reader,JsValue>{
        if !size.is_finite()||size.fract()!=0.0||size<32.0||size>MAX as f64{return Err("Choose a 7z file between 32 bytes and 64 GiB.".into());}
        let mut reader=sevenz_rust2::ArchiveReader::new(std::io::BufReader::with_capacity(1024*1024,Input{size:size as u64,position:0,read}),sevenz_rust2::Password::empty()).map_err(|e|JsValue::from_str(&format!("Could not open 7z. Encrypted or unsupported archives must be extracted first: {e}")))?;reader.set_thread_count(1);Ok(reader)
    }
    fn manifest(reader:&Reader)->Result<serde_json::Value,JsValue>{
        let mut seen=std::collections::HashSet::new();let mut total=0u64;let mut entries=Vec::new();
        if reader.archive().files.len()>1000{return Err("The 7z contains too many files (maximum 1,000).".into());}
        for entry in &reader.archive().files {
            let name=entry.name.replace('\\',"/");
            if name.starts_with('/')||name.contains(':')||name.chars().any(char::is_control)||name.split('/').any(|s|s==".."||s==".")||!seen.insert(name.to_lowercase()){return Err("The 7z contains unsafe or duplicate file paths.".into());}
            let kind=(entry.windows_attributes>>16)&0xf000;
            if entry.is_anti_item||entry.windows_attributes&0x400!=0||!matches!(kind,0|0x8000|0x4000){return Err("Links and special files are not supported in 7z imports.".into());}
            total=total.checked_add(entry.size).ok_or_else(||JsValue::from_str("7z size overflow"))?;if total>MAX{return Err("The 7z expands beyond the 64 GiB limit.".into());}
            entries.push(serde_json::json!({"name":name,"size":entry.size,"directory":entry.is_directory}));
        }
        Ok(serde_json::json!({"entries":entries,"total":total}))
    }
    #[wasm_bindgen(js_name=ironwoodSevenZipList)]
    pub fn list(size:f64,read:js_sys::Function)->Result<String,JsValue>{let reader=reader(size,read)?;Ok(manifest(&reader)?.to_string())}
    #[wasm_bindgen(js_name=ironwoodSevenZipExtract)]
    pub fn extract(size:f64,read:js_sys::Function,write:js_sys::Function,progress:js_sys::Function)->Result<(),JsValue>{
        let mut reader=reader(size,read)?;let total=manifest(&reader)?["total"].as_u64().unwrap();let mut expanded=0u64;let mut buffer=vec![0u8;1024*1024];
        reader.for_each_entries(|entry,data|{
            let name=entry.name.replace('\\',"/");let mut offset=0u64;
            loop {let n=data.read(&mut buffer)?;if n==0{break;}offset+=n as u64;expanded+=n as u64;if offset>entry.size||expanded>total{return Err(sevenz_rust2::Error::Other("7z expanded beyond its declared size".into()));}
                write.call3(&JsValue::NULL,&JsValue::from_str(&name),&JsValue::from_f64((offset-n as u64) as f64),&js_sys::Uint8Array::from(&buffer[..n])).map_err(|_|sevenz_rust2::Error::Other("Could not write temporary import data; check browser storage space".into()))?;
                progress.call2(&JsValue::NULL,&JsValue::from_f64(expanded as f64),&JsValue::from_f64(total as f64)).map_err(|_|sevenz_rust2::Error::Other("7z extraction cancelled".into()))?;
            }
            if offset!=entry.size{return Err(sevenz_rust2::Error::Other("Truncated 7z entry".into()));}Ok(true)
        }).map_err(|e|JsValue::from_str(&format!("7z extraction/integrity check failed: {e}")))?;Ok(())
    }
}
