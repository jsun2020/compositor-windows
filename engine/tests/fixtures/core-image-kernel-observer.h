#import <Foundation/Foundation.h>
FOUNDATION_EXPORT void P7InstallKernelObserver(void);
FOUNDATION_EXPORT void P7RemoveKernelObserver(void);
FOUNDATION_EXPORT void P7SetCaptureLabel(const char *label);
FOUNDATION_EXPORT NSData *P7CaptureJSON(void);
