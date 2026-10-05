using System;
using System.IO;
using System.Threading;
using System.Runtime.InteropServices;
using System.Windows.Forms;
using System.Web.Script.Serialization;

// Controlled contention only. Never reads, clears or writes clipboard contents.
class ClipboardReadOnlyLocker {
  [DllImport("user32.dll",SetLastError=true)] static extern bool OpenClipboard(IntPtr window);
  [DllImport("user32.dll")] static extern bool CloseClipboard();
  [DllImport("user32.dll")] static extern uint GetClipboardSequenceNumber();
  [STAThread] static int Main(string[] args){
    try{
      if(args.Length<1||args.Length>2)throw new ArgumentException("Expected an evidence directory and optional hold milliseconds");
      int hold=args.Length==2?int.Parse(args[1]):5000;if(hold<50||hold>5000)throw new ArgumentOutOfRangeException("hold");
      var root=Path.GetFullPath(args[0]);var json=new JavaScriptSerializer();
      using(var form=new Form()){
        var hwnd=form.Handle;
        if(!OpenClipboard(hwnd))throw new InvalidOperationException("Read-only lock refused: "+Marshal.GetLastWin32Error());
        var before=GetClipboardSequenceNumber();
        try{
          var ready=json.Serialize(new{readOnly=true,clipboardPayloadAccess=false,clipboardMutation=false,pid=System.Diagnostics.Process.GetCurrentProcess().Id,window=hwnd.ToInt64(),sequence=before,holdMs=hold,utc=DateTime.UtcNow.ToString("o")});
          var readyPath=Path.Combine(root,"lock-ready.json");
          using(var file=new StreamWriter(new FileStream(readyPath+".tmp",FileMode.CreateNew,FileAccess.Write))){file.Write(ready);}
          File.Move(readyPath+".tmp",readyPath);
          Thread.Sleep(hold);
        }finally{CloseClipboard();}
        File.WriteAllText(Path.Combine(root,"lock-closed.json"),json.Serialize(new{readOnly=true,sequenceBefore=before,sequenceAfter=GetClipboardSequenceNumber(),utc=DateTime.UtcNow.ToString("o")}));
      }
      return 0;
    }catch(Exception e){Console.Error.WriteLine(e.GetType().Name+": "+e.Message);return 1;}
  }
}
