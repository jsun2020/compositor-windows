using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Security.Cryptography;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;
using System.Windows.Forms;

// Explicit recovery only. No original acceptance helper or test is changed.
// Prepare the same WinForms HGLOBAL serialization locally before EmptyClipboard.
class ClipboardTextNativeRecovery {
  class Entry { public string name; public string kind; public string text; public string[] strings; public byte[] bytes; }
  class Snapshot { public int schema; public List<Entry> entries; }
  class Block { public string name; public uint id; public IntPtr handle; public int length; }
  static JavaScriptSerializer json=new JavaScriptSerializer(){MaxJsonLength=64*1024*1024};
  static readonly byte[] entropy=Encoding.UTF8.GetBytes("Compositor.Phase5.ClipboardRecovery.v1");
  static readonly Dictionary<string,uint> allowed=new Dictionary<string,uint>{{"Text",1},{"OEMText",7},{"UnicodeText",13},{"Locale",16}};
  [DllImport("kernel32.dll",SetLastError=true)] static extern IntPtr GlobalAlloc(uint flags,UIntPtr size);
  [DllImport("kernel32.dll",SetLastError=true)] static extern IntPtr GlobalLock(IntPtr handle);
  [DllImport("kernel32.dll",SetLastError=true)] static extern bool GlobalUnlock(IntPtr handle);
  [DllImport("kernel32.dll",SetLastError=true)] static extern UIntPtr GlobalSize(IntPtr handle);
  [DllImport("kernel32.dll",SetLastError=true)] static extern IntPtr GlobalFree(IntPtr handle);
  [DllImport("kernel32.dll")] static extern void SetLastError(uint error);
  [DllImport("ole32.dll")] static extern void ReleaseStgMedium(ref STGMEDIUM medium);
  [DllImport("user32.dll",SetLastError=true)] static extern bool OpenClipboard(IntPtr window);
  [DllImport("user32.dll",SetLastError=true)] static extern bool EmptyClipboard();
  [DllImport("user32.dll",SetLastError=true)] static extern IntPtr SetClipboardData(uint format,IntPtr handle);
  [DllImport("user32.dll",SetLastError=true)] static extern bool CloseClipboard();
  [DllImport("user32.dll")] static extern uint GetClipboardSequenceNumber();
  static Exception NativeError(string operation){return new InvalidOperationException(operation+" failed; Win32="+Marshal.GetLastWin32Error());}
  static void Validate(Snapshot snapshot){
    if(snapshot==null||snapshot.schema!=1||snapshot.entries==null||snapshot.entries.Count!=4)throw new InvalidDataException("Recovery requires exactly four standard text formats");
    var names=new HashSet<string>();
    foreach(var e in snapshot.entries){
      if(e==null||e.name==null||!allowed.ContainsKey(e.name)||!names.Add(e.name))throw new InvalidDataException("Unsupported or duplicate format; nothing mutated");
      if(e.name=="Locale"){
        if((e.kind!="stream"&&e.kind!="bytes")||e.bytes==null||e.bytes.Length!=4||e.text!=null||e.strings!=null)throw new InvalidDataException("Invalid Locale data; nothing mutated");
      }else if(e.kind!="string"||e.text==null||e.text.Length>8*1024*1024||e.text.IndexOf('\0')>=0||e.bytes!=null||e.strings!=null)throw new InvalidDataException("Invalid text data; nothing mutated");
    }
  }
  static Snapshot Read(string file){
    var plain=ProtectedData.Unprotect(File.ReadAllBytes(file),entropy,DataProtectionScope.CurrentUser);
    try{var snapshot=json.Deserialize<Snapshot>(Encoding.UTF8.GetString(plain));Validate(snapshot);return snapshot;}
    finally{Array.Clear(plain,0,plain.Length);}
  }
  static byte[] ReadGlobal(IntPtr handle){
    ulong size=GlobalSize(handle).ToUInt64();if(size==0||size>64*1024*1024)throw new InvalidDataException("Invalid serialized memory size");
    var pointer=GlobalLock(handle);if(pointer==IntPtr.Zero)throw NativeError("GlobalLock");
    try{var bytes=new byte[(int)size];Marshal.Copy(pointer,bytes,0,bytes.Length);return bytes;}finally{GlobalUnlock(handle);}
  }
  static IntPtr CopyGlobal(byte[] bytes){
    var handle=GlobalAlloc(2,(UIntPtr)(uint)bytes.Length);if(handle==IntPtr.Zero)throw NativeError("GlobalAlloc");
    var pointer=GlobalLock(handle);if(pointer==IntPtr.Zero){GlobalFree(handle);throw NativeError("GlobalLock");}
    try{Marshal.Copy(bytes,0,pointer,bytes.Length);}catch{GlobalUnlock(handle);GlobalFree(handle);throw;}
    GlobalUnlock(handle);return handle;
  }
  static void Free(List<Block> blocks){foreach(var block in blocks)if(block.handle!=IntPtr.Zero){GlobalFree(block.handle);block.handle=IntPtr.Zero;}}
  static List<Block> Prepare(Snapshot snapshot){
    Validate(snapshot);var data=new DataObject();var streams=new List<MemoryStream>();var blocks=new List<Block>();
    try{
      foreach(var e in snapshot.entries){
        object value=e.text;
        if(e.kind=="stream"){var stream=new MemoryStream(e.bytes);streams.Add(stream);value=stream;}else if(e.kind=="bytes")value=e.bytes;
        data.SetData(e.name,false,value);
      }
      var com=(System.Runtime.InteropServices.ComTypes.IDataObject)data;
      foreach(var e in snapshot.entries){
        uint id=allowed[e.name];if(DataFormats.GetFormat(e.name).Id!=id)throw new InvalidDataException("Unexpected standard format identifier");
        var format=new FORMATETC{cfFormat=(short)id,dwAspect=DVASPECT.DVASPECT_CONTENT,lindex=-1,ptd=IntPtr.Zero,tymed=TYMED.TYMED_HGLOBAL};
        STGMEDIUM medium;com.GetData(ref format,out medium);
        try{
          if(medium.tymed!=TYMED.TYMED_HGLOBAL||medium.unionmember==IntPtr.Zero)throw new InvalidDataException("Local serialization did not return HGLOBAL");
          var bytes=ReadGlobal(medium.unionmember);
          try{
            if(e.name=="Locale"&&(bytes.Length!=4||!bytes.SequenceEqual(e.bytes)))throw new InvalidDataException("Serialized Locale differs");
            if(e.name=="UnicodeText"&&(bytes.Length<2||Encoding.Unicode.GetString(bytes).TrimEnd('\0')!=e.text))throw new InvalidDataException("Serialized Unicode text differs");
            var copy=CopyGlobal(bytes);blocks.Add(new Block{name=e.name,id=id,handle=copy,length=bytes.Length});
            var copied=ReadGlobal(copy);try{if(!bytes.SequenceEqual(copied))throw new InvalidDataException("Copied memory differs");}finally{Array.Clear(copied,0,copied.Length);}
          }finally{Array.Clear(bytes,0,bytes.Length);}
        }finally{ReleaseStgMedium(ref medium);}
      }
      return blocks;
    }catch{Free(blocks);throw;}finally{foreach(var stream in streams)stream.Dispose();}
  }
  static Snapshot Synthetic(){return new Snapshot{schema=1,entries=new List<Entry>{new Entry{name="Text",kind="string",text="native recovery"},new Entry{name="OEMText",kind="string",text="native recovery"},new Entry{name="UnicodeText",kind="string",text="A\U0001F600\u4E2D\u6587B"},new Entry{name="Locale",kind="stream",bytes=new byte[]{9,4,0,0}}}};}
  static int SelfTest(){
    var blocks=Prepare(Synthetic());try{if(blocks.Count!=4)throw new InvalidDataException("Missing prepared format");}finally{Free(blocks);}
    var duplicate=Synthetic();duplicate.entries[0].name="OEMText";bool duplicates=false;try{Validate(duplicate);}catch(InvalidDataException){duplicates=true;}
    var unsupported=Synthetic();unsupported.entries[0].name="Bitmap";bool formats=false;try{Validate(unsupported);}catch(InvalidDataException){formats=true;}
    var invalid=Synthetic();invalid.entries[2].text="has\0embedded";bool nul=false;try{Validate(invalid);}catch(InvalidDataException){nul=true;}
    if(!duplicates||!formats||!nul)throw new InvalidDataException("Invalid snapshot accepted");
    Console.WriteLine(json.Serialize(new{selfTest=true,systemClipboardAccess=false,systemClipboardMutation=false,localComSerialization=true,exactCopiedMemory=true,unicodeVerified=true,localeVerified=true,duplicateRefused=duplicates,unsupportedRefused=formats,embeddedNullRefused=nul,acceptance=false}));return 0;
  }
  [STAThread] static int Main(string[] args){
    bool emptied=false;int published=0;bool opened=false;List<Block> blocks=null;
    try{
      if(args.Length==1&&args[0]=="self-test")return SelfTest();
      if(args.Length!=2||(args[0]!="prepare"&&args[0]!="restore"))throw new ArgumentException("Usage: self-test | prepare|restore ORIGINAL_DPAPI");
      blocks=Prepare(Read(Path.GetFullPath(args[1])));
      if(args[0]=="prepare"){Console.WriteLine(json.Serialize(new{preparedFormats=blocks.Select(b=>b.name).ToArray(),systemClipboardAccess=false,systemClipboardMutation=false,acceptance=false}));return 0;}
      using(var owner=new Form()){
        var window=owner.Handle;var timer=Stopwatch.StartNew();
        while(true){
          SetLastError(0);if(OpenClipboard(window)){opened=true;break;}
          int error=Marshal.GetLastWin32Error();long remaining=250-timer.ElapsedMilliseconds;
          if((error!=0&&error!=5&&error!=170)||remaining<=0)throw new InvalidOperationException("OpenClipboard failed before mutation; Win32="+error);
          Thread.Sleep((int)Math.Min(10,remaining));
        }
        try{
          if(!EmptyClipboard())throw NativeError("EmptyClipboard");emptied=true;
          foreach(var block in blocks){if(SetClipboardData(block.id,block.handle)==IntPtr.Zero)throw NativeError("SetClipboardData format="+block.id);block.handle=IntPtr.Zero;published++;}
        }finally{if(opened){bool closed=CloseClipboard();opened=false;if(!closed)throw NativeError("CloseClipboard");}}
      }
      Console.WriteLine(json.Serialize(new{recoveryOnly=true,preparedFormats=blocks.Count,publishedFormats=published,emptied=emptied,sequence=GetClipboardSequenceNumber(),immediateNativeData=true,acceptance=false}));return 0;
    }catch(Exception e){Console.Error.WriteLine(json.Serialize(new{errorType=e.GetType().Name,error=e.Message,recoveryOnly=true,emptied=emptied,publishedFormats=published,acceptance=false}));return 1;}
    finally{if(opened)CloseClipboard();if(blocks!=null)Free(blocks);}
  }
}
