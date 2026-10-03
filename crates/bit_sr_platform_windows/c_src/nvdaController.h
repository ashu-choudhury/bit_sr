

/* this ALWAYS GENERATED file contains the definitions for the interfaces */


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

#pragma warning( disable: 4049 )  /* more than 64k source lines */


/* verify that the <rpcndr.h> version is high enough to compile this file*/
#ifndef __REQUIRED_RPCNDR_H_VERSION__
#define __REQUIRED_RPCNDR_H_VERSION__ 475
#endif

#include "rpc.h"
#include "rpcndr.h"

#ifndef __RPCNDR_H_VERSION__
#error this stub requires an updated version of <rpcndr.h>
#endif /* __RPCNDR_H_VERSION__ */


#ifndef __nvdaController_h__
#define __nvdaController_h__

#if defined(_MSC_VER) && (_MSC_VER >= 1020)
#pragma once
#endif

#ifndef DECLSPEC_XFGVIRT
#if defined(_CONTROL_FLOW_GUARD_XFG)
#define DECLSPEC_XFGVIRT(base, func) __declspec(xfg_virtual(base, func))
#else
#define DECLSPEC_XFGVIRT(base, func)
#endif
#endif

/* Forward Declarations */ 

#ifdef __cplusplus
extern "C"{
#endif 


/* interface __MIDL_itf_nvdaController_0000_0000 */
/* [local] */ 

/*
This file is a part of the NVDA project.
URL: http://www.nvda-project.org/
Copyright 2006-2023 NV Access Limited, Leonard de Ruijter.
This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Lesser General Public License version 2.1, as published by
the Free Software Foundation.
This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
This license can be found at:
http://www.gnu.org/licenses/old-licenses/lgpl-2.1.html
*/
typedef /* [v1_enum] */ 
enum tagSPEECH_PRIORITY
    {
        SPEECH_PRIORITY_NORMAL	= 0,
        SPEECH_PRIORITY_NEXT	= 1,
        SPEECH_PRIORITY_NOW	= 2
    } 	SPEECH_PRIORITY;

typedef /* [v1_enum] */ 
enum tagSYMBOL_LEVEL
    {
        SYMBOL_LEVEL_NONE	= 0,
        SYMBOL_LEVEL_SOME	= 100,
        SYMBOL_LEVEL_MOST	= 200,
        SYMBOL_LEVEL_ALL	= 300,
        SYMBOL_LEVEL_CHAR	= 1000,
        SYMBOL_LEVEL_UNCHANGED	= -1
    } 	SYMBOL_LEVEL;

typedef error_status_t ( __stdcall *onSsmlMarkReachedFuncType )( 
    /* [string][in] */ const wchar_t *mark);

error_status_t __stdcall setOnSsmlMarkReachedCallback( 
    onSsmlMarkReachedFuncType callback);



extern RPC_IF_HANDLE __MIDL_itf_nvdaController_0000_0000_v0_0_c_ifspec;
extern RPC_IF_HANDLE __MIDL_itf_nvdaController_0000_0000_v0_0_s_ifspec;

#ifndef __NvdaController_INTERFACE_DEFINED__
#define __NvdaController_INTERFACE_DEFINED__

/* interface NvdaController */
/* [version][uuid] */ 

error_status_t __stdcall testIfRunning( 
    /* [in] */ handle_t IDL_handle);

error_status_t __stdcall speakText( 
    /* [in] */ handle_t IDL_handle,
    /* [string][in] */ const wchar_t *text);

error_status_t __stdcall cancelSpeech( 
    /* [in] */ handle_t IDL_handle);

error_status_t __stdcall brailleMessage( 
    /* [in] */ handle_t IDL_handle,
    /* [string][in] */ const wchar_t *message);



extern RPC_IF_HANDLE NvdaController_v1_0_c_ifspec;
extern RPC_IF_HANDLE NvdaController_v1_0_s_ifspec;
#endif /* __NvdaController_INTERFACE_DEFINED__ */

#ifndef __NvdaController2_INTERFACE_DEFINED__
#define __NvdaController2_INTERFACE_DEFINED__

/* interface NvdaController2 */
/* [version][uuid] */ 

error_status_t __stdcall getProcessId( 
    /* [in] */ handle_t IDL_handle,
    /* [out] */ unsigned long *pid);

error_status_t __stdcall speakSsml( 
    /* [in] */ handle_t IDL_handle,
    /* [string][in] */ const wchar_t *ssml,
    /* [defaultvalue][in] */ const SYMBOL_LEVEL symbolLevel,
    /* [defaultvalue][in] */ const SPEECH_PRIORITY priority,
    /* [defaultvalue][in] */ const boolean asynchronous);

/* [callback] */ error_status_t __stdcall onSsmlMarkReached( 
    /* [string][in] */ const wchar_t *mark);



extern RPC_IF_HANDLE NvdaController2_v1_0_c_ifspec;
extern RPC_IF_HANDLE NvdaController2_v1_0_s_ifspec;
#endif /* __NvdaController2_INTERFACE_DEFINED__ */

#ifndef __NvdaController3_INTERFACE_DEFINED__
#define __NvdaController3_INTERFACE_DEFINED__

/* interface NvdaController3 */
/* [version][uuid] */ 

error_status_t __stdcall isSpeaking( 
    /* [in] */ handle_t IDL_handle,
    /* [out] */ boolean *speaking);



extern RPC_IF_HANDLE NvdaController3_v1_0_c_ifspec;
extern RPC_IF_HANDLE NvdaController3_v1_0_s_ifspec;
#endif /* __NvdaController3_INTERFACE_DEFINED__ */

/* Additional Prototypes for ALL interfaces */

/* end of Additional Prototypes */

#ifdef __cplusplus
}
#endif

#endif


