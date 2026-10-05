using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;
using System.Web.Script.Serialization;

// Diagnostic metadata only. Never opens, reads, empties or writes the clipboard.
class ClipboardOwnershipProbe {
  [DllImport("user32.dll", SetLastError=true)] static extern IntPtr GetOpenClipboardWindow();
  [DllImport("user32.dll", SetLastError=true)] static extern IntPtr GetClipboardOwner();
  [DllImport("user32.dll", SetLastError=true)] static extern uint GetClipboardSequenceNumber();
  [DllImport("user32.dll", SetLastError=true)] static extern uint GetWindowThreadProcessId(IntPtr window, out uint pid);
  [DllImport("kernel32.dll")] static extern void SetLastError(uint code);
  static JavaScriptSerializer json=new JavaScriptSerializer();
  static object Window(IntPtr handle) {
    uint pid=0; uint tid=handle==IntPtr.Zero?0:GetWindowThreadProcessId(handle,out pid);
    string name=null; int? session=null;
    if(pid!=0)try{using(var p=Process.GetProcessById((int)pid)){name=p.ProcessName;session=p.SessionId;}}catch{}
    return new{window=handle.ToInt64(),pid=pid,threadId=tid,processName=name,session=session};
  }
  static int Main(string[] args) {
    try{return Run(args);}catch(Exception e){Console.Error.WriteLine(e.GetType().Name+": "+e.Message);return 1;}
  }
  static int Run(string[] args) {
    if(args.Length!=2)throw new ArgumentException("Usage: ClipboardOwnershipProbe.exe OUTPUT_JSONL SECONDS (1..600); OUTPUT_JSONL.stop stops early");
    int seconds=int.Parse(args[1]);if(seconds<1||seconds>600)throw new ArgumentOutOfRangeException("seconds");
    var clock=Stopwatch.StartNew(); long samples=0; long writes=0;
    string prior=null;long lastWritten=-1000;
    // CreateNew preserves every previous diagnostic; refusal does not touch clipboard.
    using(var output=new StreamWriter(new FileStream(Path.GetFullPath(args[0]),FileMode.CreateNew,FileAccess.Write,FileShare.Read))){
      output.AutoFlush=true;
      output.WriteLine(json.Serialize(new{eventType="start",utc=DateTime.UtcNow.ToString("o"),diagnosticOnly=true,systemClipboardPayloadAccess=false,systemClipboardMutation=false,pollIntervalMs=20,seconds=seconds,zeroLockWindowDoesNotProveUnlocked=true,ownerIsLastWriterNotNecessarilyLocker=true}));
      while(clock.ElapsedMilliseconds<seconds*1000L&&!File.Exists(args[0]+".stop")){
        SetLastError(0);var open=GetOpenClipboardWindow();var openError=Marshal.GetLastWin32Error();
        SetLastError(0);var owner=GetClipboardOwner();var ownerError=Marshal.GetLastWin32Error();
        SetLastError(0);var sequence=GetClipboardSequenceNumber();var sequenceError=Marshal.GetLastWin32Error();
        samples++;
        var state=open.ToInt64()+":"+owner.ToInt64()+":"+sequence+":"+openError+":"+ownerError+":"+sequenceError;
        if(state!=prior||clock.ElapsedMilliseconds-lastWritten>=1000){
          output.WriteLine(json.Serialize(new{eventType="sample",utc=DateTime.UtcNow.ToString("o"),elapsedMs=clock.ElapsedMilliseconds,sample=samples,openWindow=Window(open),lastWriterWindow=Window(owner),sequence=sequence,openWindowError=openError,ownerError=ownerError,sequenceError=sequenceError}));
          prior=state;lastWritten=clock.ElapsedMilliseconds;writes++;
        }
        Thread.Sleep(20);
      }
      output.WriteLine(json.Serialize(new{eventType="end",utc=DateTime.UtcNow.ToString("o"),elapsedMs=clock.ElapsedMilliseconds,totalSamples=samples,recordedSamples=writes,stopFile=File.Exists(args[0]+".stop"),diagnosticOnly=true,clipboardAcceptance=false}));
    }
    return 0;
  }
}
