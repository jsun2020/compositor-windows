// Isolated camera-sensor decoder using the unmodified LibRaw 0.22.2 Win64 SDK.
// LibRaw is distributed under its CDDL 1.0 option; see the bundled notices/source.
#include <libraw/libraw.h>
#include <windows.h>
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstdint>
#include <iostream>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>

static void check(int status) { if(status!=LIBRAW_SUCCESS)throw std::runtime_error(libraw_strerror(status)); }
static void size_check(unsigned w,unsigned h) {
  if(!w||!h||w>30000||h>30000||uint64_t(w)*h>100000000)throw std::runtime_error("RAW dimensions exceed 30,000 pixels per side or 100 MP");
}
static std::string quote(const char* text) {
  std::ostringstream out;out<<'"';for(const unsigned char* p=(const unsigned char*)text;*p;++p){
    if(*p=='"'||*p=='\\')out<<'\\'<<char(*p);else if(*p<32||*p>=127){char escape[7];snprintf(escape,sizeof(escape),"\\u%04x",*p);out<<escape;}else out<<char(*p);
  }out<<'"';return out.str();
}
// Kelvin is an approximation for the UI. Exact Reset uses the camera's own
// multipliers. A custom temperature changes their ratio relative to this estimate.
static std::vector<double> illuminant(double kelvin) {
  double t=kelvin/100.;double r=t<=66?255:329.698727446*pow(t-60,-.1332047592);
  double g=t<=66?99.4708025861*log(t)-161.1195681661:288.1221695283*pow(t-60,-.0755148492);
  double b=t>=66?255:t<=19?0:138.5177312231*log(t-10)-305.0447927307;
  return {std::clamp(r/255.,.02,1.),std::clamp(g/255.,.02,1.),std::clamp(b/255.,.02,1.)};
}
static double shot_temperature(const LibRaw& raw) {
  const auto& c=raw.imgdata.color;double red=c.cam_mul[0],blue=c.cam_mul[2];
  if(!std::isfinite(red)||!std::isfinite(blue)||red<=0||blue<=0)return 5000.;
  double ratio=blue/red;double best=5000,delta=1e30;
  for(int t=2000;t<=15000;t+=25){auto color=illuminant(t);double d=std::abs(std::log(color[0]/color[2])-std::log(ratio));if(d<delta){best=t;delta=d;}}
  return best;
}
static double number(const wchar_t* text,double low,double high) {
  wchar_t* end=nullptr;double value=wcstod(text,&end);if(!end||*end||!std::isfinite(value)||value<low||value>high)throw std::runtime_error("Invalid RAW develop control");return value;
}
int wmain(int argc,wchar_t** argv) {
  try {
    if(argc!=3&&argc!=9)throw std::runtime_error("usage: --inspect input | --develop input output exposure temperature tint tone preview");
    bool inspect=std::wstring(argv[1])==L"--inspect";
    if((inspect&&argc!=3)||(!inspect&&(std::wstring(argv[1])!=L"--develop"||argc!=9)))throw std::runtime_error("Invalid operation");
    WIN32_FILE_ATTRIBUTE_DATA attrs{};if(!GetFileAttributesExW(argv[2],GetFileExInfoStandard,&attrs)||attrs.dwFileAttributes&FILE_ATTRIBUTE_DIRECTORY)throw std::runtime_error("RAW file is unreadable");
    if((uint64_t(attrs.nFileSizeHigh)<<32|attrs.nFileSizeLow)>512ull*1024*1024)throw std::runtime_error("RAW file exceeds 512 MiB");
    LibRaw raw;raw.imgdata.rawparams.max_raw_memory_mb=1536;check(raw.open_file(argv[2]));
    size_check(raw.imgdata.sizes.raw_width,raw.imgdata.sizes.raw_height);size_check(raw.imgdata.sizes.width,raw.imgdata.sizes.height);
    if(!raw.imgdata.idata.raw_count)throw std::runtime_error("The file contains no camera sensor image");
    double shot=shot_temperature(raw);auto& s=raw.imgdata.sizes;
    std::ostringstream metadata;metadata<<"{\"decoder\":\"LibRaw "<<LibRaw::version()<<"\",\"width\":"<<s.width<<",\"height\":"<<s.height<<",\"make\":"<<quote(raw.imgdata.idata.make)<<",\"model\":"<<quote(raw.imgdata.idata.model)<<",\"asShotTemperature\":"<<shot<<",\"asShotTint\":0,\"whiteBalanceEstimate\":true,\"sensorDecoded\":"<<(inspect?"false":"true")<<"}";
    if(inspect){std::cout<<metadata.str();return 0;}
    double exposure=number(argv[4],-5,5),temperature=number(argv[5],2000,15000),tint=number(argv[6],-150,150),tone=number(argv[7],0,1);bool preview=number(argv[8],0,1)!=0;
    auto& p=raw.imgdata.params;p.output_color=1;p.output_bps=8;p.no_auto_bright=1;p.use_camera_wb=1;p.user_qual=3;
    p.half_size=preview&&std::max(s.width,s.height)>2048;p.bright=float(std::exp2(exposure));
    // Windows interpretation of tone strength, not Apple's proprietary boost curve.
    p.gamm[0]=1./(1.+1.4*tone);p.gamm[1]=tone>0?12.92:1.;
    if(temperature!=shot||tint!=0){auto from=illuminant(shot),to=illuminant(temperature);for(int i=0;i<4;++i){int ch=i==3?1:i;double base=raw.imgdata.color.cam_mul[i];if(!std::isfinite(base)||base<=0)base=1.;p.user_mul[i]=float(base*from[ch]/to[ch]*(ch==1?std::exp2(-tint/150.):1.));}p.use_camera_wb=0;}
    check(raw.unpack());check(raw.dcraw_process());int status=0;libraw_processed_image_t* image=raw.dcraw_make_mem_image(&status);check(status);
    if(!image)throw std::runtime_error("RAW decoder produced no image");
    try {
      if(image->type!=LIBRAW_IMAGE_BITMAP||image->colors!=3||image->bits!=8)throw std::runtime_error("Unsupported decoded RAW layout");
      size_check(image->width,image->height);uint64_t pixels=uint64_t(image->width)*image->height;
      if(image->data_size!=pixels*3)throw std::runtime_error("Decoded RAW pixel length is inconsistent");
      HANDLE output=CreateFileW(argv[3],GENERIC_WRITE,0,nullptr,CREATE_NEW,FILE_ATTRIBUTE_NORMAL,nullptr);
      if(output==INVALID_HANDLE_VALUE)throw std::runtime_error("Cannot create a new RAW output");
      auto write=[&](const void* bytes,DWORD size){DWORD written=0;if(!WriteFile(output,bytes,size,&written,nullptr)||written!=size)throw std::runtime_error("RAW output write failed");};
      try {uint32_t header[4]={0x37574152,image->width,image->height,0};write(header,sizeof(header));std::vector<unsigned char> row(size_t(image->width)*4);
        for(unsigned y=0;y<image->height;++y){for(unsigned x=0;x<image->width;++x){size_t src=(size_t(y)*image->width+x)*3;row[x*4]=image->data[src];row[x*4+1]=image->data[src+1];row[x*4+2]=image->data[src+2];row[x*4+3]=255;}write(row.data(),DWORD(row.size()));}CloseHandle(output);
      }catch(...){CloseHandle(output);throw;}
      std::cout<<metadata.str();LibRaw::dcraw_clear_mem(image);return 0;
    }catch(...){LibRaw::dcraw_clear_mem(image);throw;}
  }catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}catch(...){std::cerr<<"RAW decoding failed\n";return 1;}
}
