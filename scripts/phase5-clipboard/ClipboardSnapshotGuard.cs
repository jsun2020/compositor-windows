using System;
using System.IO;
using System.Drawing;
using System.Drawing.Imaging;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Windows.Forms;
using System.Web.Script.Serialization;
using System.Runtime.InteropServices;
using System.Collections.Generic;

// A recovery backup, separate from the unchanged acceptance helper/protocol.
// Snapshot/verify/self-test never writes the system clipboard. Restore is an
// explicit operation and can never change an acceptance failure into a pass.
class ClipboardSnapshotGuard {
  [DllImport("user32.dll")] static extern uint GetClipboardSequenceNumber();
  class Entry { public string name; public string kind; public string text; public string[] strings; public byte[] bytes; }
  class Snapshot { public int schema=1; public List<Entry> entries=new List<Entry>(); }
  static JavaScriptSerializer json=new JavaScriptSerializer(){MaxJsonLength=64*1024*1024};
  static readonly byte[] entropy=Encoding.UTF8.GetBytes("Compositor.Phase5.ClipboardRecovery.v1");
  static byte[] Serialize(Snapshot snapshot){return Encoding.UTF8.GetBytes(json.Serialize(snapshot));}
  static Snapshot Deserialize(byte[] bytes){var snapshot=json.Deserialize<Snapshot>(Encoding.UTF8.GetString(bytes));if(snapshot==null||snapshot.schema!=1||snapshot.entries==null)throw new InvalidDataException("Unsupported backup schema");return snapshot;}
  static Entry Freeze(string name,object value){
    var e=new Entry{name=name};
    if(value is string){e.kind="string";e.text=(string)value;}
    else if(value is string[]){e.kind="strings";e.strings=(string[])((string[])value).Clone();}
    else if(value is byte[]){e.kind="bytes";e.bytes=(byte[])((byte[])value).Clone();}
    else if(value is MemoryStream){e.kind="stream";e.bytes=((MemoryStream)value).ToArray();}
    else if(value is Bitmap){
      var bitmap=(Bitmap)value;
      if(bitmap.PixelFormat!=PixelFormat.Format32bppArgb&&bitmap.PixelFormat!=PixelFormat.Format24bppRgb)
        throw new InvalidDataException("Bitmap format cannot be verified losslessly; no clipboard mutation attempted");
      e.kind="bitmap";using(var stream=new MemoryStream()){bitmap.Save(stream,ImageFormat.Png);e.bytes=stream.ToArray();}
      using(var stream=new MemoryStream(e.bytes))using(var decoded=new Bitmap(stream)){
        if(bitmap.Size!=decoded.Size)throw new InvalidDataException("Bitmap backup dimensions differ");
        for(int y=0;y<bitmap.Height;y++)for(int x=0;x<bitmap.Width;x++)
          if(bitmap.GetPixel(x,y).ToArgb()!=decoded.GetPixel(x,y).ToArgb())throw new InvalidDataException("Bitmap backup pixels differ");
      }
    }else throw new InvalidDataException("Unsupported clipboard value type; no clipboard mutation attempted");
    return e;
  }
  static object Thaw(Entry e){
    switch(e.kind){
      case "string":return e.text;
      case "strings":return e.strings;
      case "bytes":return e.bytes;
      case "stream":return new MemoryStream(e.bytes);
      case "bitmap":using(var stream=new MemoryStream(e.bytes))using(var bitmap=new Bitmap(stream))return bitmap.Clone();
      default:throw new InvalidDataException("Unknown backup value type");
    }
  }
  static void Verify(Snapshot snapshot){
    var names=new HashSet<string>();
    foreach(var entry in snapshot.entries){
      if(String.IsNullOrEmpty(entry.name)||!names.Add(entry.name))throw new InvalidDataException("Invalid backup format list");
      var value=Thaw(entry);try{
        var roundtrip=Freeze(entry.name,value);
        if(entry.kind!=roundtrip.kind||entry.text!=roundtrip.text||
           (entry.strings!=null&&!entry.strings.SequenceEqual(roundtrip.strings))||
           (entry.bytes!=null&&entry.kind!="bitmap"&&!entry.bytes.SequenceEqual(roundtrip.bytes)))
          throw new InvalidDataException("Backup roundtrip differs");
      }finally{var disposable=value as IDisposable;if(disposable!=null)disposable.Dispose();}
    }
  }
  static Snapshot ReadBackup(string file){byte[] plain=ProtectedData.Unprotect(File.ReadAllBytes(file),entropy,DataProtectionScope.CurrentUser);try{return Deserialize(plain);}finally{Array.Clear(plain,0,plain.Length);}}
  static void WriteBackup(string file,Snapshot snapshot){
    var plain=Serialize(snapshot);byte[] encrypted;
    try{encrypted=ProtectedData.Protect(plain,entropy,DataProtectionScope.CurrentUser);}finally{Array.Clear(plain,0,plain.Length);}
    using(var output=new FileStream(file,FileMode.CreateNew,FileAccess.Write,FileShare.None)){output.Write(encrypted,0,encrypted.Length);}
    var verified=ReadBackup(file);Verify(verified);
    if(json.Serialize(snapshot)!=json.Serialize(verified))throw new InvalidDataException("Encrypted snapshot differs after decrypt");
  }
  static int SelfTest(string directory){
    Directory.CreateDirectory(directory);var snapshot=new Snapshot();
    snapshot.entries.Add(Freeze("UnicodeText","A\U0001F600\u4E2D\u6587B clipboard"));
    snapshot.entries.Add(Freeze("Text","text"));snapshot.entries.Add(Freeze("Locale",new MemoryStream(new byte[]{9,4,0,0})));
    snapshot.entries.Add(Freeze("BytePayload",new byte[]{0,1,255,128}));snapshot.entries.Add(Freeze("FileDrop",new string[]{"synthetic-a","synthetic-b"}));
    using(var bitmap=new Bitmap(3,2,PixelFormat.Format32bppArgb)){bitmap.SetPixel(1,1,Color.FromArgb(128,128,0,128));snapshot.entries.Add(Freeze("Bitmap",bitmap));}
    Verify(snapshot);var backup=Path.Combine(directory,"synthetic.dpapi");WriteBackup(backup,snapshot);
    var restored=ReadBackup(backup);Verify(restored);
    if(json.Serialize(snapshot)!=json.Serialize(restored))throw new InvalidDataException("Synthetic backup changed");
    var damaged=File.ReadAllBytes(backup);damaged[damaged.Length/2]^=1;
    bool refused=false;try{ProtectedData.Unprotect(damaged,entropy,DataProtectionScope.CurrentUser);}catch(CryptographicException){refused=true;}
    if(!refused)throw new InvalidDataException("Tampered backup accepted");
    bool unsupported=false;try{Freeze("unsupported",new object());}catch(InvalidDataException){unsupported=true;}
    if(!unsupported)throw new InvalidDataException("Unsupported type accepted");
    bool existingRefused=false;try{WriteBackup(backup,snapshot);}catch(IOException){existingRefused=true;}
    if(!existingRefused)throw new InvalidDataException("Existing backup overwritten");
    Console.WriteLine(json.Serialize(new{selfTest=true,systemClipboardAccess=false,systemClipboardMutation=false,supportedKinds=5,syntheticEntries=6,exactRoundtrip=true,tamperRefused=true,unsupportedTypeRefused=true,existingOutputRefused=true,acceptance=false}));return 0;
  }
  [STAThread] static int Main(string[] args){
    try{
      if(args.Length!=2)throw new ArgumentException("Usage: ClipboardSnapshotGuard.exe snapshot|verify|restore|self-test PATH");
      if(args[0]=="self-test")return SelfTest(Path.GetFullPath(args[1]));
      if(args[0]=="check-sequence"){
        var ready=json.Deserialize<Dictionary<string,object>>(File.ReadAllText(Path.GetFullPath(args[1])));
        var expected=Convert.ToUInt32(ready["sequence"]);var current=GetClipboardSequenceNumber();
        if(expected==0||current!=expected)throw new InvalidOperationException("Clipboard changed since backup; protocol must not start");
        Console.WriteLine("Original snapshot sequence still matches; no clipboard payload read or mutation.");return 0;
      }
      if(args[0]=="snapshot"){
        var directory=Path.GetFullPath(args[1]);Directory.CreateDirectory(directory);
        var sequenceBefore=GetClipboardSequenceNumber();if(sequenceBefore==0)throw new InvalidOperationException("Clipboard sequence unavailable; nothing mutated");
        var source=Clipboard.GetDataObject();if(source==null)throw new InvalidOperationException("Clipboard snapshot unavailable; nothing mutated");
        var snapshot=new Snapshot();
        foreach(var name in source.GetFormats(false)){if(name=="DataObject"||name=="Ole Private Data")continue;snapshot.entries.Add(Freeze(name,source.GetData(name,false)));}
        var sequenceAfter=GetClipboardSequenceNumber();if(sequenceAfter!=sequenceBefore)throw new InvalidOperationException("Clipboard changed during backup; nothing mutated");
        Verify(snapshot);WriteBackup(Path.Combine(directory,"original.dpapi"),snapshot);
        if(GetClipboardSequenceNumber()!=sequenceAfter)throw new InvalidOperationException("Clipboard changed during backup verification; nothing mutated");
        File.WriteAllText(Path.Combine(directory,"snapshot-ready.json"),json.Serialize(new{encrypted=true,scope="CurrentUser",sequence=sequenceAfter,formats=snapshot.entries.Select(e=>e.name).ToArray(),systemClipboardMutation=false,verified=true,acceptance=false,utc=DateTime.UtcNow.ToString("o")}));
        Console.WriteLine("Encrypted original snapshot verified; system clipboard was not mutated.");return 0;
      }
      if(args[0]=="verify"||args[0]=="restore"){
        var snapshot=ReadBackup(Path.GetFullPath(args[1]));Verify(snapshot);
        if(args[0]=="verify"){Console.WriteLine("Encrypted snapshot decrypts and validates; system clipboard was not accessed.");return 0;}
        var data=new DataObject();foreach(var e in snapshot.entries)data.SetData(e.name,false,Thaw(e));
        Clipboard.SetDataObject(data,true);
        Console.WriteLine("Recovery operation completed; original acceptance failures remain failed.");return 0;
      }
      throw new ArgumentException("Unknown operation");
    }catch(Exception e){Console.Error.WriteLine(e.GetType().Name+": "+e.Message);return 1;}
  }
}
