using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;
using System.Web.Script.Serialization;

// Read-only environmental evidence, separate from original acceptance assertions.
// Never reads clipboard contents, sends input, unlocks a session or changes policy.
class Phase5SessionGate {
  [DllImport("wtsapi32.dll", EntryPoint="WTSQuerySessionInformationW", SetLastError=true)]
  static extern bool Query(IntPtr server, int session, int info, out IntPtr buffer, out int bytes);
  [DllImport("wtsapi32.dll")] static extern void WTSFreeMemory(IntPtr buffer);
  static readonly JavaScriptSerializer json = new JavaScriptSerializer();
  class Sample {
    public string utc;
    public int session, level, returnedSession, state, flags, error, bytes;
    public bool success, unlocked, readOnly=true, clipboardAccess=false;
  }
  static Sample Read() {
    var sample=new Sample {utc=DateTime.UtcNow.ToString("o"), session=Process.GetCurrentProcess().SessionId};
    IntPtr buffer; int bytes;
    sample.success=Query(IntPtr.Zero,sample.session,25,out buffer,out bytes);
    sample.error=Marshal.GetLastWin32Error();sample.bytes=bytes;
    if(!sample.success) return sample;
    try {
      // WTSINFOEX Level is followed by its 8-byte-aligned Level1 union.
      // Verify both identifiers before interpreting SessionFlags (Windows 8+).
      if(bytes<20){sample.success=false;return sample;}
      sample.level=Marshal.ReadInt32(buffer,0);
      sample.returnedSession=Marshal.ReadInt32(buffer,8);
      sample.state=Marshal.ReadInt32(buffer,12);
      sample.flags=Marshal.ReadInt32(buffer,16);
      var os=Environment.OSVersion.Version;
      if(sample.level!=1 || sample.returnedSession!=sample.session ||
         (os.Major==6 && os.Minor==1) || (sample.flags!=0 && sample.flags!=1)){
        sample.success=false;return sample;
      }
      sample.unlocked=sample.flags==1 && sample.state==0;
      return sample;
    } finally {WTSFreeMemory(buffer);}
  }
  static int Main(string[] args) {
    try {
      if(args.Length==1 && args[0]=="check") {
        var sample=Read();Console.WriteLine(json.Serialize(sample));
        return sample.success && sample.unlocked ? 0 : 2;
      }
      if(args.Length!=3 || args[0]!="monitor")
        throw new ArgumentException("Usage: Phase5SessionGate.exe check | monitor NEW_JSONL SECONDS");
      int seconds;
      if(!Int32.TryParse(args[2],out seconds) || seconds<1 || seconds>1800)
        throw new ArgumentException("Monitor duration must be 1..1800 seconds");
      var file=Path.GetFullPath(args[1]);
      if(File.Exists(file)||File.Exists(file+".stop"))throw new IOException("Preserve earlier session evidence");
      var clock=Stopwatch.StartNew();int count=0;bool valid=true;
      using(var writer=new StreamWriter(new FileStream(file,FileMode.CreateNew,FileAccess.Write,FileShare.Read))) {
        do {
          var sample=Read();count++;valid=valid && sample.success && sample.unlocked;
          writer.WriteLine(json.Serialize(sample));writer.Flush();
          if(File.Exists(file+".stop"))break;
          Thread.Sleep(200);
        } while(clock.Elapsed.TotalSeconds<seconds);
      }
      File.WriteAllText(file+".summary.json",json.Serialize(new {readOnly=true,clipboardAccess=false,samples=count,elapsedMs=clock.ElapsedMilliseconds,intervalMs=200,allSamplesUnlocked=valid,doesNotProveStateBetweenSamples=true}));
      return valid ? 0 : 2;
    } catch(Exception e) {Console.Error.WriteLine(e.GetType().Name+": "+e.Message);return 1;}
  }
}
