use crate::*;
use uuid::Uuid;

impl Engine {
    /// Install a native-decoded PDF/HEIF as one atomic edit. PDF input is page order;
    /// page one is inserted last so it is the topmost layer and the active page.
    pub fn import_pages(&mut self,id:Option<Uuid>,images:Vec<(String,Vec<u8>)>,resolution:f64,at:Option<Point>)->Result<Uuid,CommandError>{
        if images.is_empty()||images.len()>MAX_LAYERS||!resolution.is_finite()||!(1.0..=9600.).contains(&resolution){return Err(CommandError::Argument("Invalid image page metadata".into()));}
        if at.is_some_and(|p|!p.x.is_finite()||!p.y.is_finite()){return Err(CommandError::Argument("Invalid import position".into()));}
        let (mut used,layers)=if let Some(id)=id{
            let s=self.session(id)?;if s.preview.is_some()||s.floating.is_some(){return Err(CommandError::Refused("Apply or cancel the current edit before importing".into()));}
            (s.document.used_pixels(),s.document.layers.len())
        }else{(0,0)};
        if layers+images.len()>MAX_LAYERS{return Err(ProjectError::TooLarge.into());}
        let mut bytes=0u64;
        for(name,data)in &images{
            if name.trim().is_empty()||name.len()>16384{return Err(CommandError::Argument("Invalid page name".into()));}
            bytes+=data.len()as u64;if bytes>MAX_ASSET_BYTES{return Err(ProjectError::TooLarge.into());}
            let reader=image::ImageReader::with_format(std::io::Cursor::new(data),image::ImageFormat::Png);
            let(w,h)=reader.into_dimensions().map_err(|_|ImportError::Unreadable)?;
            if w==0||h==0||w>30000||h>30000{return Err(ImportError::TooLarge.into());}
            used+=u64::from(w)*u64::from(h);if used>MAX_PIXELS{return Err(ImportError::OverBudget.into());}
        }
        let mut pages=Vec::with_capacity(images.len());
        for(name,data)in images{pages.push((name,decode_image(&data)?.raster));}
        let mut doc=if let Some(id)=id{self.document(id).unwrap().clone()}else{let first=&pages[0].1;let mut d=Document::new(first.width,first.height);d.resolution=resolution;d};
        for(name,raster)in pages.into_iter().rev(){ops::layers::import_raster(&mut doc,raster,&name,at)?;}
        doc.manifest().validate()?;
        if let Some(id)=id{self.edit(id,move|next,_|{*next=doc;Ok(Dirty::everything())})?;Ok(id)}
        else{let handle=self.insert_unsaved_document(doc);Ok(handle)}
    }
}
