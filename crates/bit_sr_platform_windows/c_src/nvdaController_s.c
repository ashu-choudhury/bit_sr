

/* this ALWAYS GENERATED file contains the RPC server stubs */


 /* File created by MIDL compiler version 8.01.0628 */
/* at Tue Jan 19 08:44:07 2038
 */
/* Compiler settings for references\nvda\nvdaHelper\interfaces\nvdaController\nvdaController.idl:
    Oicf, W1, Zp8, env=Win64 (32b run), target_arch=AMD64 8.01.0628 
    protocol : dce , ms_ext, app_config, c_ext, robust
    error checks: allocation ref bounds_check enum stub_data 
    VC __declspec() decoration level: 
         __declspec(uuid()), __declspec(selectany), __declspec(novtable)
         DECLSPEC_UUID(), MIDL_INTERFACE()
*/
/* @@MIDL_FILE_HEADING(  ) */

#if defined(_M_AMD64)


#pragma warning( disable: 4049 )  /* more than 64k source lines */
#if _MSC_VER >= 1200
#pragma warning(push)
#endif

#pragma warning( disable: 4211 )  /* redefine extern to static */
#pragma warning( disable: 4232 )  /* dllimport identity*/
#pragma warning( disable: 4024 )  /* array to pointer mapping*/
#pragma warning( disable: 4100 ) /* unreferenced arguments in x86 call */

#pragma optimize("", off ) 

#include <stdlib.h>
#include <string.h>
#include "nvdaController.h"

void* __RPC_USER MIDL_user_allocate(size_t size) {
    return malloc(size);
}

void __RPC_USER MIDL_user_free(void* p) {
    free(p);
}

#define TYPE_FORMAT_STRING_SIZE   15                                
#define PROC_FORMAT_STRING_SIZE   339                               
#define EXPR_FORMAT_STRING_SIZE   1                                 
#define TRANSMIT_AS_TABLE_SIZE    0            
#define WIRE_MARSHAL_TABLE_SIZE   0            

typedef struct _nvdaController_MIDL_TYPE_FORMAT_STRING
    {
    short          Pad;
    unsigned char  Format[ TYPE_FORMAT_STRING_SIZE ];
    } nvdaController_MIDL_TYPE_FORMAT_STRING;

typedef struct _nvdaController_MIDL_PROC_FORMAT_STRING
    {
    short          Pad;
    unsigned char  Format[ PROC_FORMAT_STRING_SIZE ];
    } nvdaController_MIDL_PROC_FORMAT_STRING;

typedef struct _nvdaController_MIDL_EXPR_FORMAT_STRING
    {
    long          Pad;
    unsigned char  Format[ EXPR_FORMAT_STRING_SIZE ];
    } nvdaController_MIDL_EXPR_FORMAT_STRING;


static const RPC_SYNTAX_IDENTIFIER  _RpcTransferSyntax_2_0 = 
{{0x8A885D04,0x1CEB,0x11C9,{0x9F,0xE8,0x08,0x00,0x2B,0x10,0x48,0x60}},{2,0}};

#if defined(_CONTROL_FLOW_GUARD_XFG)
#define XFG_TRAMPOLINES(ObjectType)\
NDR_SHAREABLE unsigned long ObjectType ## _UserSize_XFG(unsigned long * pFlags, unsigned long Offset, void * pObject)\
{\
return  ObjectType ## _UserSize(pFlags, Offset, (ObjectType *)pObject);\
}\
NDR_SHAREABLE unsigned char * ObjectType ## _UserMarshal_XFG(unsigned long * pFlags, unsigned char * pBuffer, void * pObject)\
{\
return ObjectType ## _UserMarshal(pFlags, pBuffer, (ObjectType *)pObject);\
}\
NDR_SHAREABLE unsigned char * ObjectType ## _UserUnmarshal_XFG(unsigned long * pFlags, unsigned char * pBuffer, void * pObject)\
{\
return ObjectType ## _UserUnmarshal(pFlags, pBuffer, (ObjectType *)pObject);\
}\
NDR_SHAREABLE void ObjectType ## _UserFree_XFG(unsigned long * pFlags, void * pObject)\
{\
ObjectType ## _UserFree(pFlags, (ObjectType *)pObject);\
}
#define XFG_TRAMPOLINES64(ObjectType)\
NDR_SHAREABLE unsigned long ObjectType ## _UserSize64_XFG(unsigned long * pFlags, unsigned long Offset, void * pObject)\
{\
return  ObjectType ## _UserSize64(pFlags, Offset, (ObjectType *)pObject);\
}\
NDR_SHAREABLE unsigned char * ObjectType ## _UserMarshal64_XFG(unsigned long * pFlags, unsigned char * pBuffer, void * pObject)\
{\
return ObjectType ## _UserMarshal64(pFlags, pBuffer, (ObjectType *)pObject);\
}\
NDR_SHAREABLE unsigned char * ObjectType ## _UserUnmarshal64_XFG(unsigned long * pFlags, unsigned char * pBuffer, void * pObject)\
{\
return ObjectType ## _UserUnmarshal64(pFlags, pBuffer, (ObjectType *)pObject);\
}\
NDR_SHAREABLE void ObjectType ## _UserFree64_XFG(unsigned long * pFlags, void * pObject)\
{\
ObjectType ## _UserFree64(pFlags, (ObjectType *)pObject);\
}
#define XFG_BIND_TRAMPOLINES(HandleType, ObjectType)\
static void* ObjectType ## _bind_XFG(HandleType pObject)\
{\
return ObjectType ## _bind((ObjectType) pObject);\
}\
static void ObjectType ## _unbind_XFG(HandleType pObject, handle_t ServerHandle)\
{\
ObjectType ## _unbind((ObjectType) pObject, ServerHandle);\
}
#define XFG_TRAMPOLINE_FPTR(Function) Function ## _XFG
#define XFG_TRAMPOLINE_FPTR_DEPENDENT_SYMBOL(Symbol) Symbol ## _XFG
#else
#define XFG_TRAMPOLINES(ObjectType)
#define XFG_TRAMPOLINES64(ObjectType)
#define XFG_BIND_TRAMPOLINES(HandleType, ObjectType)
#define XFG_TRAMPOLINE_FPTR(Function) Function
#define XFG_TRAMPOLINE_FPTR_DEPENDENT_SYMBOL(Symbol) Symbol
#endif

extern const nvdaController_MIDL_TYPE_FORMAT_STRING nvdaController__MIDL_TypeFormatString;
extern const nvdaController_MIDL_PROC_FORMAT_STRING nvdaController__MIDL_ProcFormatString;
extern const nvdaController_MIDL_EXPR_FORMAT_STRING nvdaController__MIDL_ExprFormatString;

/* Standard interface: __MIDL_itf_nvdaController_0000_0000, ver. 0.0,
   GUID={0x00000000,0x0000,0x0000,{0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00}} */


/* Standard interface: NvdaController, ver. 1.0,
   GUID={0xDFF50B99,0xF7FD,0x4ca7,{0xA8,0x2C,0xDA,0xEB,0x3E,0x02,0x52,0x95}} */


extern const MIDL_SERVER_INFO NvdaController_ServerInfo;

extern const RPC_DISPATCH_TABLE NvdaController_v1_0_DispatchTable;

static const RPC_SERVER_INTERFACE NvdaController___RpcServerInterface =
    {
    sizeof(RPC_SERVER_INTERFACE),
    {{0xDFF50B99,0xF7FD,0x4ca7,{0xA8,0x2C,0xDA,0xEB,0x3E,0x02,0x52,0x95}},{1,0}},
    {{0x8A885D04,0x1CEB,0x11C9,{0x9F,0xE8,0x08,0x00,0x2B,0x10,0x48,0x60}},{2,0}},
    (RPC_DISPATCH_TABLE*)&NvdaController_v1_0_DispatchTable,
    0,
    0,
    0,
    &NvdaController_ServerInfo,
    0x04000000
    };
RPC_IF_HANDLE NvdaController_v1_0_s_ifspec = (RPC_IF_HANDLE)& NvdaController___RpcServerInterface;
#ifdef __cplusplus
namespace {
#endif

extern const MIDL_STUB_DESC NvdaController_StubDesc;
#ifdef __cplusplus
}
#endif


/* Standard interface: NvdaController2, ver. 1.0,
   GUID={0x3D168D45,0xCB58,0x4270,{0x82,0x57,0x4E,0x0B,0xE5,0x15,0xD5,0x57}} */


extern const MIDL_SERVER_INFO NvdaController2_ServerInfo;

extern const RPC_DISPATCH_TABLE NvdaController2_v1_0_DispatchTable;

static const RPC_SERVER_INTERFACE NvdaController2___RpcServerInterface =
    {
    sizeof(RPC_SERVER_INTERFACE),
    {{0x3D168D45,0xCB58,0x4270,{0x82,0x57,0x4E,0x0B,0xE5,0x15,0xD5,0x57}},{1,0}},
    {{0x8A885D04,0x1CEB,0x11C9,{0x9F,0xE8,0x08,0x00,0x2B,0x10,0x48,0x60}},{2,0}},
    (RPC_DISPATCH_TABLE*)&NvdaController2_v1_0_DispatchTable,
    0,
    0,
    0,
    &NvdaController2_ServerInfo,
    0x04000000
    };
RPC_IF_HANDLE NvdaController2_v1_0_s_ifspec = (RPC_IF_HANDLE)& NvdaController2___RpcServerInterface;
#ifdef __cplusplus
namespace {
#endif

extern const MIDL_STUB_DESC NvdaController2_StubDesc;
#ifdef __cplusplus
}
#endif

 extern const MIDL_STUBLESS_PROXY_INFO NvdaController2_ProxyInfo;

/* [callback] */ error_status_t __stdcall onSsmlMarkReached( 
    /* [string][in] */ const wchar_t *mark)
{

    CLIENT_CALL_RETURN _RetVal;

    _RetVal = NdrClientCall2(
                  ( PMIDL_STUB_DESC  )&NvdaController2_StubDesc,
                  (PFORMAT_STRING) &nvdaController__MIDL_ProcFormatString.Format[258],
                  mark);
    return ( error_status_t  )_RetVal.Simple;
    
}


/* Standard interface: NvdaController3, ver. 1.0,
   GUID={0x019e4216,0xa9a1,0x7540,{0xa2,0x17,0xe1,0xa2,0xd2,0x79,0x30,0x25}} */


extern const MIDL_SERVER_INFO NvdaController3_ServerInfo;

extern const RPC_DISPATCH_TABLE NvdaController3_v1_0_DispatchTable;

static const RPC_SERVER_INTERFACE NvdaController3___RpcServerInterface =
    {
    sizeof(RPC_SERVER_INTERFACE),
    {{0x019e4216,0xa9a1,0x7540,{0xa2,0x17,0xe1,0xa2,0xd2,0x79,0x30,0x25}},{1,0}},
    {{0x8A885D04,0x1CEB,0x11C9,{0x9F,0xE8,0x08,0x00,0x2B,0x10,0x48,0x60}},{2,0}},
    (RPC_DISPATCH_TABLE*)&NvdaController3_v1_0_DispatchTable,
    0,
    0,
    0,
    &NvdaController3_ServerInfo,
    0x04000000
    };
RPC_IF_HANDLE NvdaController3_v1_0_s_ifspec = (RPC_IF_HANDLE)& NvdaController3___RpcServerInterface;
#ifdef __cplusplus
namespace {
#endif

extern const MIDL_STUB_DESC NvdaController3_StubDesc;
#ifdef __cplusplus
}
#endif


#if !defined(__RPC_WIN64__)
#error  Invalid build platform for this stub.
#endif

#if !(TARGET_IS_NT50_OR_LATER)
#error You need Windows 2000 or later to run this stub because it uses these features:
#error   /robust command line switch.
#error However, your C/C++ compilation flags indicate you intend to run this app on earlier systems.
#error This app will fail with the RPC_X_WRONG_STUB_VERSION error.
#endif


const nvdaController_MIDL_PROC_FORMAT_STRING nvdaController__MIDL_ProcFormatString =
    {
        0,
        {

	/* Procedure testIfRunning */

			0x0,		/* 0 */
			0x48,		/* Old Flags:  */
/*  2 */	NdrFcLong( 0x0 ),	/* 0 */
/*  6 */	NdrFcShort( 0x0 ),	/* 0 */
/*  8 */	NdrFcShort( 0x10 ),	/* x86 Stack size/offset = 16 */
/* 10 */	0x32,		/* FC_BIND_PRIMITIVE */
			0x0,		/* 0 */
/* 12 */	NdrFcShort( 0x0 ),	/* x86 Stack size/offset = 0 */
/* 14 */	NdrFcShort( 0x0 ),	/* 0 */
/* 16 */	NdrFcShort( 0x8 ),	/* 8 */
/* 18 */	0x44,		/* Oi2 Flags:  has return, has ext, */
			0x1,		/* 1 */
/* 20 */	0xa,		/* 10 */
			0x1,		/* Ext Flags:  new corr desc, */
/* 22 */	NdrFcShort( 0x0 ),	/* 0 */
/* 24 */	NdrFcShort( 0x0 ),	/* 0 */
/* 26 */	NdrFcShort( 0x0 ),	/* 0 */
/* 28 */	NdrFcShort( 0x0 ),	/* 0 */

	/* Return value */

/* 30 */	NdrFcShort( 0x70 ),	/* Flags:  out, return, base type, */
/* 32 */	NdrFcShort( 0x8 ),	/* x86 Stack size/offset = 8 */
/* 34 */	0x10,		/* FC_ERROR_STATUS_T */
			0x0,		/* 0 */

	/* Procedure speakText */

/* 36 */	0x0,		/* 0 */
			0x48,		/* Old Flags:  */
/* 38 */	NdrFcLong( 0x0 ),	/* 0 */
/* 42 */	NdrFcShort( 0x1 ),	/* 1 */
/* 44 */	NdrFcShort( 0x18 ),	/* x86 Stack size/offset = 24 */
/* 46 */	0x32,		/* FC_BIND_PRIMITIVE */
			0x0,		/* 0 */
/* 48 */	NdrFcShort( 0x0 ),	/* x86 Stack size/offset = 0 */
/* 50 */	NdrFcShort( 0x0 ),	/* 0 */
/* 52 */	NdrFcShort( 0x8 ),	/* 8 */
/* 54 */	0x46,		/* Oi2 Flags:  clt must size, has return, has ext, */
			0x2,		/* 2 */
/* 56 */	0xa,		/* 10 */
			0x1,		/* Ext Flags:  new corr desc, */
/* 58 */	NdrFcShort( 0x0 ),	/* 0 */
/* 60 */	NdrFcShort( 0x0 ),	/* 0 */
/* 62 */	NdrFcShort( 0x0 ),	/* 0 */
/* 64 */	NdrFcShort( 0x0 ),	/* 0 */

	/* Parameter text */

/* 66 */	NdrFcShort( 0x10b ),	/* Flags:  must size, must free, in, simple ref, */
/* 68 */	NdrFcShort( 0x8 ),	/* x86 Stack size/offset = 8 */
/* 70 */	NdrFcShort( 0x4 ),	/* Type Offset=4 */

	/* Return value */

/* 72 */	NdrFcShort( 0x70 ),	/* Flags:  out, return, base type, */
/* 74 */	NdrFcShort( 0x10 ),	/* x86 Stack size/offset = 16 */
/* 76 */	0x10,		/* FC_ERROR_STATUS_T */
			0x0,		/* 0 */

	/* Procedure cancelSpeech */

/* 78 */	0x0,		/* 0 */
			0x48,		/* Old Flags:  */
/* 80 */	NdrFcLong( 0x0 ),	/* 0 */
/* 84 */	NdrFcShort( 0x2 ),	/* 2 */
/* 86 */	NdrFcShort( 0x10 ),	/* x86 Stack size/offset = 16 */
/* 88 */	0x32,		/* FC_BIND_PRIMITIVE */
			0x0,		/* 0 */
/* 90 */	NdrFcShort( 0x0 ),	/* x86 Stack size/offset = 0 */
/* 92 */	NdrFcShort( 0x0 ),	/* 0 */
/* 94 */	NdrFcShort( 0x8 ),	/* 8 */
/* 96 */	0x44,		/* Oi2 Flags:  has return, has ext, */
			0x1,		/* 1 */
/* 98 */	0xa,		/* 10 */
			0x1,		/* Ext Flags:  new corr desc, */
/* 100 */	NdrFcShort( 0x0 ),	/* 0 */
/* 102 */	NdrFcShort( 0x0 ),	/* 0 */
/* 104 */	NdrFcShort( 0x0 ),	/* 0 */
/* 106 */	NdrFcShort( 0x0 ),	/* 0 */

	/* Return value */

/* 108 */	NdrFcShort( 0x70 ),	/* Flags:  out, return, base type, */
/* 110 */	NdrFcShort( 0x8 ),	/* x86 Stack size/offset = 8 */
/* 112 */	0x10,		/* FC_ERROR_STATUS_T */
			0x0,		/* 0 */

	/* Procedure brailleMessage */

/* 114 */	0x0,		/* 0 */
			0x48,		/* Old Flags:  */
/* 116 */	NdrFcLong( 0x0 ),	/* 0 */
/* 120 */	NdrFcShort( 0x3 ),	/* 3 */
/* 122 */	NdrFcShort( 0x18 ),	/* x86 Stack size/offset = 24 */
/* 124 */	0x32,		/* FC_BIND_PRIMITIVE */
			0x0,		/* 0 */
/* 126 */	NdrFcShort( 0x0 ),	/* x86 Stack size/offset = 0 */
/* 128 */	NdrFcShort( 0x0 ),	/* 0 */
/* 130 */	NdrFcShort( 0x8 ),	/* 8 */
/* 132 */	0x46,		/* Oi2 Flags:  clt must size, has return, has ext, */
			0x2,		/* 2 */
/* 134 */	0xa,		/* 10 */
			0x1,		/* Ext Flags:  new corr desc, */
/* 136 */	NdrFcShort( 0x0 ),	/* 0 */
/* 138 */	NdrFcShort( 0x0 ),	/* 0 */
/* 140 */	NdrFcShort( 0x0 ),	/* 0 */
/* 142 */	NdrFcShort( 0x0 ),	/* 0 */

	/* Parameter message */

/* 144 */	NdrFcShort( 0x10b ),	/* Flags:  must size, must free, in, simple ref, */
/* 146 */	NdrFcShort( 0x8 ),	/* x86 Stack size/offset = 8 */
/* 148 */	NdrFcShort( 0x4 ),	/* Type Offset=4 */

	/* Return value */

/* 150 */	NdrFcShort( 0x70 ),	/* Flags:  out, return, base type, */
/* 152 */	NdrFcShort( 0x10 ),	/* x86 Stack size/offset = 16 */
/* 154 */	0x10,		/* FC_ERROR_STATUS_T */
			0x0,		/* 0 */

	/* Procedure getProcessId */

/* 156 */	0x0,		/* 0 */
			0x48,		/* Old Flags:  */
/* 158 */	NdrFcLong( 0x0 ),	/* 0 */
/* 162 */	NdrFcShort( 0x0 ),	/* 0 */
/* 164 */	NdrFcShort( 0x18 ),	/* x86 Stack size/offset = 24 */
/* 166 */	0x32,		/* FC_BIND_PRIMITIVE */
			0x0,		/* 0 */
/* 168 */	NdrFcShort( 0x0 ),	/* x86 Stack size/offset = 0 */
/* 170 */	NdrFcShort( 0x0 ),	/* 0 */
/* 172 */	NdrFcShort( 0x24 ),	/* 36 */
/* 174 */	0x44,		/* Oi2 Flags:  has return, has ext, */
			0x2,		/* 2 */
/* 176 */	0xa,		/* 10 */
			0x1,		/* Ext Flags:  new corr desc, */
/* 178 */	NdrFcShort( 0x0 ),	/* 0 */
/* 180 */	NdrFcShort( 0x0 ),	/* 0 */
/* 182 */	NdrFcShort( 0x0 ),	/* 0 */
/* 184 */	NdrFcShort( 0x0 ),	/* 0 */

	/* Parameter pid */

/* 186 */	NdrFcShort( 0x2150 ),	/* Flags:  out, base type, simple ref, srv alloc size=8 */
/* 188 */	NdrFcShort( 0x8 ),	/* x86 Stack size/offset = 8 */
/* 190 */	0x8,		/* FC_LONG */
			0x0,		/* 0 */

	/* Return value */

/* 192 */	NdrFcShort( 0x70 ),	/* Flags:  out, return, base type, */
/* 194 */	NdrFcShort( 0x10 ),	/* x86 Stack size/offset = 16 */
/* 196 */	0x10,		/* FC_ERROR_STATUS_T */
			0x0,		/* 0 */

	/* Procedure speakSsml */

/* 198 */	0x0,		/* 0 */
			0x48,		/* Old Flags:  */
/* 200 */	NdrFcLong( 0x0 ),	/* 0 */
/* 204 */	NdrFcShort( 0x1 ),	/* 1 */
/* 206 */	NdrFcShort( 0x30 ),	/* x86 Stack size/offset = 48 */
/* 208 */	0x32,		/* FC_BIND_PRIMITIVE */
			0x0,		/* 0 */
/* 210 */	NdrFcShort( 0x0 ),	/* x86 Stack size/offset = 0 */
/* 212 */	NdrFcShort( 0x15 ),	/* 21 */
/* 214 */	NdrFcShort( 0x8 ),	/* 8 */
/* 216 */	0x46,		/* Oi2 Flags:  clt must size, has return, has ext, */
			0x5,		/* 5 */
/* 218 */	0xa,		/* 10 */
			0x1,		/* Ext Flags:  new corr desc, */
/* 220 */	NdrFcShort( 0x0 ),	/* 0 */
/* 222 */	NdrFcShort( 0x0 ),	/* 0 */
/* 224 */	NdrFcShort( 0x0 ),	/* 0 */
/* 226 */	NdrFcShort( 0x0 ),	/* 0 */

	/* Parameter ssml */

/* 228 */	NdrFcShort( 0x10b ),	/* Flags:  must size, must free, in, simple ref, */
/* 230 */	NdrFcShort( 0x8 ),	/* x86 Stack size/offset = 8 */
/* 232 */	NdrFcShort( 0x4 ),	/* Type Offset=4 */

	/* Parameter symbolLevel */

/* 234 */	NdrFcShort( 0x48 ),	/* Flags:  in, base type, */
/* 236 */	NdrFcShort( 0x10 ),	/* x86 Stack size/offset = 16 */
/* 238 */	0xe,		/* FC_ENUM32 */
			0x0,		/* 0 */

	/* Parameter priority */

/* 240 */	NdrFcShort( 0x48 ),	/* Flags:  in, base type, */
/* 242 */	NdrFcShort( 0x18 ),	/* x86 Stack size/offset = 24 */
/* 244 */	0xe,		/* FC_ENUM32 */
			0x0,		/* 0 */

	/* Parameter asynchronous */

/* 246 */	NdrFcShort( 0x48 ),	/* Flags:  in, base type, */
/* 248 */	NdrFcShort( 0x20 ),	/* x86 Stack size/offset = 32 */
/* 250 */	0x3,		/* FC_SMALL */
			0x0,		/* 0 */

	/* Return value */

/* 252 */	NdrFcShort( 0x70 ),	/* Flags:  out, return, base type, */
/* 254 */	NdrFcShort( 0x28 ),	/* x86 Stack size/offset = 40 */
/* 256 */	0x10,		/* FC_ERROR_STATUS_T */
			0x0,		/* 0 */

	/* Procedure onSsmlMarkReached */

/* 258 */	0x34,		/* FC_CALLBACK_HANDLE */
			0x48,		/* Old Flags:  */
/* 260 */	NdrFcLong( 0x0 ),	/* 0 */
/* 264 */	NdrFcShort( 0x0 ),	/* 0 */
/* 266 */	NdrFcShort( 0x10 ),	/* x86 Stack size/offset = 16 */
/* 268 */	NdrFcShort( 0x0 ),	/* 0 */
/* 270 */	NdrFcShort( 0x8 ),	/* 8 */
/* 272 */	0x46,		/* Oi2 Flags:  clt must size, has return, has ext, */
			0x2,		/* 2 */
/* 274 */	0xa,		/* 10 */
			0x1,		/* Ext Flags:  new corr desc, */
/* 276 */	NdrFcShort( 0x0 ),	/* 0 */
/* 278 */	NdrFcShort( 0x0 ),	/* 0 */
/* 280 */	NdrFcShort( 0x0 ),	/* 0 */
/* 282 */	NdrFcShort( 0x0 ),	/* 0 */

	/* Parameter mark */

/* 284 */	NdrFcShort( 0x10b ),	/* Flags:  must size, must free, in, simple ref, */
/* 286 */	NdrFcShort( 0x0 ),	/* x86 Stack size/offset = 0 */
/* 288 */	NdrFcShort( 0x4 ),	/* Type Offset=4 */

	/* Return value */

/* 290 */	NdrFcShort( 0x70 ),	/* Flags:  out, return, base type, */
/* 292 */	NdrFcShort( 0x8 ),	/* x86 Stack size/offset = 8 */
/* 294 */	0x10,		/* FC_ERROR_STATUS_T */
			0x0,		/* 0 */

	/* Procedure isSpeaking */

/* 296 */	0x0,		/* 0 */
			0x48,		/* Old Flags:  */
/* 298 */	NdrFcLong( 0x0 ),	/* 0 */
/* 302 */	NdrFcShort( 0x0 ),	/* 0 */
/* 304 */	NdrFcShort( 0x18 ),	/* x86 Stack size/offset = 24 */
/* 306 */	0x32,		/* FC_BIND_PRIMITIVE */
			0x0,		/* 0 */
/* 308 */	NdrFcShort( 0x0 ),	/* x86 Stack size/offset = 0 */
/* 310 */	NdrFcShort( 0x0 ),	/* 0 */
/* 312 */	NdrFcShort( 0x21 ),	/* 33 */
/* 314 */	0x44,		/* Oi2 Flags:  has return, has ext, */
			0x2,		/* 2 */
/* 316 */	0xa,		/* 10 */
			0x1,		/* Ext Flags:  new corr desc, */
/* 318 */	NdrFcShort( 0x0 ),	/* 0 */
/* 320 */	NdrFcShort( 0x0 ),	/* 0 */
/* 322 */	NdrFcShort( 0x0 ),	/* 0 */
/* 324 */	NdrFcShort( 0x0 ),	/* 0 */

	/* Parameter speaking */

/* 326 */	NdrFcShort( 0x2150 ),	/* Flags:  out, base type, simple ref, srv alloc size=8 */
/* 328 */	NdrFcShort( 0x8 ),	/* x86 Stack size/offset = 8 */
/* 330 */	0x3,		/* FC_SMALL */
			0x0,		/* 0 */

	/* Return value */

/* 332 */	NdrFcShort( 0x70 ),	/* Flags:  out, return, base type, */
/* 334 */	NdrFcShort( 0x10 ),	/* x86 Stack size/offset = 16 */
/* 336 */	0x10,		/* FC_ERROR_STATUS_T */
			0x0,		/* 0 */

			0x0
        }
    };

const nvdaController_MIDL_TYPE_FORMAT_STRING nvdaController__MIDL_TypeFormatString =
    {
        0,
        {
			NdrFcShort( 0x0 ),	/* 0 */
/*  2 */	
			0x11, 0x8,	/* FC_RP [simple_pointer] */
/*  4 */	
			0x25,		/* FC_C_WSTRING */
			0x5c,		/* FC_PAD */
/*  6 */	
			0x11, 0xc,	/* FC_RP [alloced_on_stack] [simple_pointer] */
/*  8 */	0x8,		/* FC_LONG */
			0x5c,		/* FC_PAD */
/* 10 */	
			0x11, 0xc,	/* FC_RP [alloced_on_stack] [simple_pointer] */
/* 12 */	0x3,		/* FC_SMALL */
			0x5c,		/* FC_PAD */

			0x0
        }
    };

static const unsigned short NvdaController_FormatStringOffsetTable[] =
    {
    0,
    36,
    78,
    114
    };


#ifdef __cplusplus
namespace {
#endif
const MIDL_STUB_DESC NvdaController_StubDesc = 
    {
    (void *)& NvdaController___RpcServerInterface,
    MIDL_user_allocate,
    MIDL_user_free,
    0,
    0,
    0,
    0,
    0,
    nvdaController__MIDL_TypeFormatString.Format,
    1, /* -error bounds_check flag */
    0x50002, /* Ndr library version */
    0,
    0x8010274, /* MIDL Version 8.1.628 */
    0,
    0,
    0,  /* notify & notify_flag routine table */
    0x1, /* MIDL flag */
    0, /* cs routines */
    0,   /* proxy/server info */
    0
    };
#ifdef __cplusplus
}
#endif

static const RPC_DISPATCH_FUNCTION NvdaController_table[] =
    {
    NdrServerCall2,
    NdrServerCall2,
    NdrServerCall2,
    NdrServerCall2,
    0
    };
const RPC_DISPATCH_TABLE NvdaController_v1_0_DispatchTable = 
    {
    4,
    (RPC_DISPATCH_FUNCTION*)NvdaController_table
    };

static const SERVER_ROUTINE NvdaController_ServerRoutineTable[] = 
    {
    (SERVER_ROUTINE)testIfRunning,
    (SERVER_ROUTINE)speakText,
    (SERVER_ROUTINE)cancelSpeech,
    (SERVER_ROUTINE)brailleMessage
    };

const MIDL_SERVER_INFO NvdaController_ServerInfo = 
    {
    &NvdaController_StubDesc,
    NvdaController_ServerRoutineTable,
    nvdaController__MIDL_ProcFormatString.Format,
    NvdaController_FormatStringOffsetTable,
    0,
    0,
    0,
    0};

static const unsigned short NvdaController2_FormatStringOffsetTable[] =
    {
    156,
    198,
    };


static const unsigned short _callbackNvdaController2_FormatStringOffsetTable[] =
    {
    258
    };


#ifdef __cplusplus
namespace {
#endif
const MIDL_STUB_DESC NvdaController2_StubDesc = 
    {
    (void *)& NvdaController2___RpcServerInterface,
    MIDL_user_allocate,
    MIDL_user_free,
    0,
    0,
    0,
    0,
    0,
    nvdaController__MIDL_TypeFormatString.Format,
    1, /* -error bounds_check flag */
    0x50002, /* Ndr library version */
    0,
    0x8010274, /* MIDL Version 8.1.628 */
    0,
    0,
    0,  /* notify & notify_flag routine table */
    0x1, /* MIDL flag */
    0, /* cs routines */
    0,   /* proxy/server info */
    0
    };
#ifdef __cplusplus
}
#endif

static const RPC_DISPATCH_FUNCTION NvdaController2_table[] =
    {
    NdrServerCall2,
    NdrServerCall2,
    0
    };
const RPC_DISPATCH_TABLE NvdaController2_v1_0_DispatchTable = 
    {
    2,
    (RPC_DISPATCH_FUNCTION*)NvdaController2_table
    };

static const SERVER_ROUTINE NvdaController2_ServerRoutineTable[] = 
    {
    (SERVER_ROUTINE)getProcessId,
    (SERVER_ROUTINE)speakSsml,
    };

const MIDL_SERVER_INFO NvdaController2_ServerInfo = 
    {
    &NvdaController2_StubDesc,
    NvdaController2_ServerRoutineTable,
    nvdaController__MIDL_ProcFormatString.Format,
    NvdaController2_FormatStringOffsetTable,
    0,
    0,
    0,
    0};

static const unsigned short NvdaController3_FormatStringOffsetTable[] =
    {
    296
    };


#ifdef __cplusplus
namespace {
#endif
const MIDL_STUB_DESC NvdaController3_StubDesc = 
    {
    (void *)& NvdaController3___RpcServerInterface,
    MIDL_user_allocate,
    MIDL_user_free,
    0,
    0,
    0,
    0,
    0,
    nvdaController__MIDL_TypeFormatString.Format,
    1, /* -error bounds_check flag */
    0x50002, /* Ndr library version */
    0,
    0x8010274, /* MIDL Version 8.1.628 */
    0,
    0,
    0,  /* notify & notify_flag routine table */
    0x1, /* MIDL flag */
    0, /* cs routines */
    0,   /* proxy/server info */
    0
    };
#ifdef __cplusplus
}
#endif

static const RPC_DISPATCH_FUNCTION NvdaController3_table[] =
    {
    NdrServerCall2,
    0
    };
const RPC_DISPATCH_TABLE NvdaController3_v1_0_DispatchTable = 
    {
    1,
    (RPC_DISPATCH_FUNCTION*)NvdaController3_table
    };

static const SERVER_ROUTINE NvdaController3_ServerRoutineTable[] = 
    {
    (SERVER_ROUTINE)isSpeaking
    };

const MIDL_SERVER_INFO NvdaController3_ServerInfo = 
    {
    &NvdaController3_StubDesc,
    NvdaController3_ServerRoutineTable,
    nvdaController__MIDL_ProcFormatString.Format,
    NvdaController3_FormatStringOffsetTable,
    0,
    0,
    0,
    0};
#if _MSC_VER >= 1200
#pragma warning(pop)
#endif


#endif /* defined(_M_AMD64)*/

