#import "core-image-kernel-observer.h"
#import <CoreImage/CoreImage.h>
#import <objc/runtime.h>
#include <string.h>
#include <stdint.h>

// Only this diagnostic process changes its method dispatch. Every observer
// calls the original implementation with the exact original arguments.
static NSLock *p7Lock;
static NSMutableArray *p7Events, *p7Installed, *p7Capabilities;
static NSString *p7Label;
static id P7Value(id value, unsigned depth) {
    if (!value) return @{ @"type": @"nil" };
    if ([value isKindOfClass:[CIVector class]]) {
        CIVector *v=value; NSMutableArray *values=[NSMutableArray array];
        for (size_t i=0;i<v.count;i++) {
            double d=[v valueAtIndex:i]; float f=(float)d;
            uint64_t db; uint32_t fb; memcpy(&db,&d,sizeof(db)); memcpy(&fb,&f,sizeof(fb));
            [values addObject:@{ @"double": @(d), @"doubleBits": [NSString stringWithFormat:@"%016llx",(unsigned long long)db],
                @"floatBits": [NSString stringWithFormat:@"%08x",fb] }];
        }
        return @{ @"type": @"CIVector", @"count": @(v.count), @"values": values };
    }
    if ([value isKindOfClass:[CIImage class]]) {
        CGRect r=[(CIImage *)value extent];
        return @{ @"type": @"CIImage", @"extentDescription": NSStringFromRect(r) };
    }
    if ([value isKindOfClass:[NSNumber class]]) return @{ @"type": @"NSNumber", @"value": value, @"objcType": @([(NSNumber *)value objCType]) };
    if ([value isKindOfClass:[NSString class]]) return @{ @"type": @"NSString", @"value": value };
    if ([value isKindOfClass:[NSArray class]] && depth<3) {
        NSMutableArray *a=[NSMutableArray array]; for (id item in (NSArray *)value) [a addObject:P7Value(item,depth+1)];
        return @{ @"type": @"NSArray", @"values": a };
    }
    return @{ @"type": NSStringFromClass([value class]), @"description": [value description] ?: @"" };
}
static void P7Record(CIKernel *kernel, NSString *entry, NSArray *arguments) {
    [p7Lock lock];
    if (p7Label.length) {
        NSMutableArray *args=[NSMutableArray array]; for (id arg in arguments ?: @[]) [args addObject:P7Value(arg,0)];
        [p7Events addObject:@{ @"label": p7Label, @"entry": entry, @"kernelClass": NSStringFromClass([kernel class]),
            @"kernelName": kernel.name ?: @"", @"arguments": args }];
    }
    [p7Lock unlock];
}

@interface CIWarpKernel (P7Observe)
- (CIImage *)p7_warp:(CGRect)extent roiCallback:(CIKernelROICallback)callback inputImage:(CIImage *)image arguments:(NSArray *)args;
@end
@implementation CIWarpKernel (P7Observe)
- (CIImage *)p7_warp:(CGRect)extent roiCallback:(CIKernelROICallback)callback inputImage:(CIImage *)image arguments:(NSArray *)args {
    P7Record(self,@"CIWarpKernel.apply",args);
    return [self p7_warp:extent roiCallback:callback inputImage:image arguments:args];
}
@end
@interface CIKernel (P7Observe)
- (CIImage *)p7_generic:(CGRect)extent roiCallback:(CIKernelROICallback)callback arguments:(NSArray *)args;
@end
@implementation CIKernel (P7Observe)
- (CIImage *)p7_generic:(CGRect)extent roiCallback:(CIKernelROICallback)callback arguments:(NSArray *)args {
    P7Record(self,@"CIKernel.apply",args);
    return [self p7_generic:extent roiCallback:callback arguments:args];
}
@end
@interface CIColorKernel (P7Observe)
- (CIImage *)p7_color:(CGRect)extent arguments:(NSArray *)args;
@end
@implementation CIColorKernel (P7Observe)
- (CIImage *)p7_color:(CGRect)extent arguments:(NSArray *)args {
    P7Record(self,@"CIColorKernel.apply",args);
    return [self p7_color:extent arguments:args];
}
@end
@interface CIFilter (P7Observe)
- (CIImage *)p7_legacy:(CIKernel *)kernel arguments:(NSArray *)args options:(NSDictionary *)options;
@end
@implementation CIFilter (P7Observe)
- (CIImage *)p7_legacy:(CIKernel *)kernel arguments:(NSArray *)args options:(NSDictionary *)options {
    P7Record(kernel,@"CIFilter.apply",args);
    return [self p7_legacy:kernel arguments:args options:options];
}
@end

static void P7Install(Class cls, SEL selector, SEL replacement) {
    Method original=class_getInstanceMethod(cls,selector), wrapper=class_getInstanceMethod(cls,replacement);
    const char *types=original ? method_getTypeEncoding(original) : "";
    BOOL compatible=original && wrapper && strcmp(types,method_getTypeEncoding(wrapper))==0;
    [p7Capabilities addObject:@{ @"class": NSStringFromClass(cls), @"selector": NSStringFromSelector(selector),
        @"typeEncoding": @(types), @"wrapperTypeEncoding": wrapper ? @(method_getTypeEncoding(wrapper)) : @"",
        @"installed": @(compatible) }];
    if (compatible) {
        method_exchangeImplementations(original,wrapper);
        [p7Installed addObject:@[[NSValue valueWithPointer:original],[NSValue valueWithPointer:wrapper]]];
    }
}
void P7InstallKernelObserver(void) {
    p7Lock=[[NSLock alloc] init]; p7Events=[NSMutableArray array]; p7Installed=[NSMutableArray array];
    p7Capabilities=[NSMutableArray array]; p7Label=@"";
    P7Install([CIWarpKernel class],@selector(applyWithExtent:roiCallback:inputImage:arguments:),@selector(p7_warp:roiCallback:inputImage:arguments:));
    P7Install([CIKernel class],@selector(applyWithExtent:roiCallback:arguments:),@selector(p7_generic:roiCallback:arguments:));
    P7Install([CIColorKernel class],@selector(applyWithExtent:arguments:),@selector(p7_color:arguments:));
    P7Install([CIFilter class],@selector(apply:arguments:options:),@selector(p7_legacy:arguments:options:));
}
void P7SetCaptureLabel(const char *label) {
    [p7Lock lock]; p7Label=label ? [NSString stringWithUTF8String:label] : @""; [p7Lock unlock];
}
void P7RemoveKernelObserver(void) {
    P7SetCaptureLabel(NULL);
    for (NSArray *pair in [p7Installed reverseObjectEnumerator]) method_exchangeImplementations([pair[0] pointerValue],[pair[1] pointerValue]);
    [p7Installed removeAllObjects];
}
NSData *P7CaptureJSON(void) {
    [p7Lock lock]; NSError *error=nil;
    NSData *data=[NSJSONSerialization dataWithJSONObject:@{ @"capabilities": p7Capabilities ?: @[], @"events": p7Events ?: @[],
        @"scope": @"Public kernel arguments in this diagnostic process; original methods forwarded unchanged and restored" } options:NSJSONWritingSortedKeys error:&error];
    [p7Lock unlock];
    if (!data) [NSException raise:@"P7SerializationFailed" format:@"%@",error];
    return data;
}
