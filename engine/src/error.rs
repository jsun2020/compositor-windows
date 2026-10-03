use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ProjectError {
    #[error("This is not a valid Compositor project, or its metadata is damaged.")]
    Invalid,
    /// The Mac's words (ProjectStore.swift:69 at v1.4.5), the upper bound read from the version this
    /// build writes so the two cannot drift apart, and which Mac saves them: a project from a newer
    /// Mac needs a newer build of this app.
    #[error("This project uses format version {0}. This app supports versions 1-{max}, which Compositor for Mac saves up to version 1.4.5.", max = crate::manifest::CURRENT_VERSION)]
    Version(u32),
    #[error("An image inside the project is missing or damaged. The current document has not been replaced.")]
    MissingImage,
    #[error("This project exceeds the supported canvas, layer, file-size, or 100-megapixel image limit.")]
    TooLarge,
    /// Over this build's pixel budget. Compositor for Mac 1.2.10 allows min(800 MP, max(200 MP,
    /// RAM / 16)) (DocumentLimits.swift:36-37); a wasm32 heap cannot hold that, so this build keeps
    /// 100 MP of layer images and 100 MP of masks, and says so (Phase 3.5b ruling).
    #[error("This project is larger than Compositor for Windows supports: its layer images, or its masks, add up to more than 100 megapixels. Compositor for Mac can open larger projects, depending on the Mac's memory.")]
    OverBudget,
    #[error("An image could not be saved. The previous project has not been replaced.")]
    Encode,
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ImportError {
    #[error("The image could not be read. It may be damaged or unavailable.")]
    Unreadable,
    #[error("Choose a JPEG, PNG, TIFF, WebP, or BMP image.")]
    Unsupported,
    #[error("This import exceeds the current 100-megapixel document budget or 30,000-pixel side limit.")]
    TooLarge,
    /// Over the single-image budget, read from the header before decoding. Worded like
    /// `ProjectError::OverBudget` (Phase 3.5b fix round 1), but about the one image being imported.
    #[error("This image is larger than Compositor for Windows supports: it has more than 100 megapixels. Compositor for Mac can open larger images, depending on the Mac's memory.")]
    OverBudget,
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ExportError {
    #[error("Image export supports canvases up to 100 megapixels and 30,000 pixels per side.")]
    TooLarge,
    #[error("The canvas could not be rendered. Try a smaller canvas.")]
    Render,
    #[error("The image could not be encoded.")]
    Encode,
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum CommandError {
    #[error("No document with that id.")]
    NoDocument,
    #[error("No layer with that id.")]
    NoLayer,
    #[error("{0}")]
    Project(#[from] ProjectError),
    #[error("{0}")]
    Import(#[from] ImportError),
    #[error("{0}")]
    Export(#[from] ExportError),
    #[error("Invalid argument: {0}")]
    Argument(String),
    /// An edit the document's state does not allow, said as the user should read it: an empty
    /// selection, a Magic Wand outline too detailed to draw (Phase 4a).
    #[error("{0}")]
    Refused(String),
}
