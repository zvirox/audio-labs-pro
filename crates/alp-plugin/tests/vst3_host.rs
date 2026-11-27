//! Integration test compiling and hosting a real C VST3 COM ABI shared library.

use alp_plugin::{ActivePlugin, PluginFormat, ProcessContext, scan_vst3_bundle};
use std::process::Command;

const MOCK_VST3_C: &str = r#"
#include <stdint.h>
#include <stdbool.h>
#include <string.h>

typedef int32_t tresult;
#define kResultOk 0
#define kNoInterface ((tresult)0x80004002L)

typedef struct { uint8_t bytes[16]; } TUID;

static const uint8_t IID_FUNKNOWN[16]     = {0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0xC0,0x00,0x00,0x00,0x00,0x00,0x00,0x46};
static const uint8_t IID_IFACTORY[16]     = {0x7A,0x4D,0x81,0x1C,0x52,0x11,0x4A,0x1F,0xAE,0xD9,0xD2,0xEE,0x0B,0x43,0xBF,0x9F};
static const uint8_t IID_IFACTORY2[16]    = {0x00,0x07,0xB6,0x50,0xF2,0x4B,0x4C,0x0B,0xA4,0x64,0xED,0xB9,0xF0,0x0B,0x2A,0xBB};
static const uint8_t IID_IPLUGINBASE[16]  = {0x22,0x88,0x8D,0xDB,0x15,0x6E,0x45,0xAE,0x83,0x58,0xB3,0x48,0x08,0x19,0x06,0x25};
static const uint8_t IID_ICOMPONENT[16]   = {0xE8,0x31,0xFF,0x31,0xF2,0xD5,0x43,0x01,0x92,0x8E,0xBB,0xEE,0x25,0x69,0x78,0x02};
static const uint8_t IID_IAUDIOPROC[16]   = {0x42,0x04,0x3F,0x99,0xB7,0xDA,0x45,0x3C,0xA5,0x69,0xE7,0x9D,0x9A,0xAE,0xC3,0x3D};
static const uint8_t IID_IEDITCTRL[16]    = {0xDC,0xD7,0xBB,0xE3,0x77,0x42,0x44,0x8D,0xA8,0x74,0xAA,0xCC,0x97,0x9C,0x75,0x9E};
static const uint8_t IID_IPLUGVIEW[16]    = {0x5B,0xC3,0x25,0x07,0xD0,0x60,0x49,0xEA,0xA6,0x15,0x1B,0x52,0x2B,0x75,0x5B,0x29};

static const uint8_t CID_MOCK[16]         = {0x11,0x22,0x33,0x44,0x55,0x66,0x77,0x88,0x99,0xAA,0xBB,0xCC,0xDD,0xEE,0xFF,0x01};

static bool iid_eq(const void* a, const void* b) { return memcmp(a, b, 16) == 0; }

static int g_module_refs = 0;
static uintptr_t g_last_parent_xid = 0;
static int g_view_attached = 0;

/* --- FUnknown / IPlugView --- */
typedef struct ViewVtbl {
    tresult (*queryInterface)(void*, const TUID*, void**);
    uint32_t (*addRef)(void*);
    uint32_t (*release)(void*);
    tresult (*isPlatformTypeSupported)(void*, const char*);
    tresult (*attached)(void*, void*, const char*);
    tresult (*removed)(void*);
    tresult (*onWheel)(void*, float);
    tresult (*onKeyDown)(void*, uint16_t, int16_t, int16_t);
    tresult (*onKeyUp)(void*, uint16_t, int16_t, int16_t);
    tresult (*getSize)(void*, int32_t*);
    tresult (*onSize)(void*, int32_t*);
    tresult (*onFocus)(void*, uint8_t);
    tresult (*setFrame)(void*, void*);
    tresult (*canResize)(void*);
    tresult (*checkSizeConstraint)(void*, int32_t*);
} ViewVtbl;

typedef struct { const ViewVtbl* vtbl; uint32_t refs; } MockView;
static tresult view_qi(void* t, const TUID* iid, void** o) {
    if (iid_eq(iid, IID_FUNKNOWN) || iid_eq(iid, IID_IPLUGVIEW)) {
        ((MockView*)t)->refs++; *o = t; return kResultOk;
    }
    *o = NULL; return kNoInterface;
}
static uint32_t view_add(void* t) { return ++((MockView*)t)->refs; }
static uint32_t view_rel(void* t) { return --((MockView*)t)->refs; }
static tresult view_plat(void* t, const char* type) {
    (void)t;
    return (type && strcmp(type, "X11EmbedWindowID") == 0) ? kResultOk : -1;
}
static tresult view_attach(void* t, void* parent, const char* type) {
    (void)t; (void)type;
    g_last_parent_xid = (uintptr_t)parent;
    g_view_attached = 1;
    return kResultOk;
}
static tresult view_rem(void* t) { (void)t; g_view_attached = 0; return kResultOk; }
static tresult view_getsize(void* t, int32_t* rect) {
    (void)t; rect[0] = 0; rect[1] = 0; rect[2] = 720; rect[3] = 420; return kResultOk;
}
static tresult view_stub_ok(void* t, ...) { (void)t; return kResultOk; }
static const ViewVtbl VIEW_VTBL = {
    view_qi, view_add, view_rel, view_plat, view_attach, view_rem,
    (void*)view_stub_ok, (void*)view_stub_ok, (void*)view_stub_ok,
    view_getsize, (void*)view_stub_ok, (void*)view_stub_ok,
    (void*)view_stub_ok, (void*)view_stub_ok, (void*)view_stub_ok
};
static MockView g_view = { &VIEW_VTBL, 0 };

/* --- Combined IComponent + IAudioProcessor + IEditController --- */
typedef struct {
    const void* comp_vtbl;
    const void* proc_vtbl;
    const void* edit_vtbl;
    uint32_t refs;
    double gain;
    int active;
    int processing;
} MockPlugin;

typedef struct {
    int32_t numChannels;
    uint64_t silenceFlags;
    float** channelBuffers32;
} AudioBusBuffers;

typedef struct {
    tresult (*queryInterface)(void*, const TUID*, void**);
    uint32_t (*addRef)(void*);
    uint32_t (*release)(void*);
    int32_t (*getParameterCount)(void*);
    void* (*getParameterData)(void*, int32_t);
} ParamChangesVtbl;

typedef struct {
    tresult (*queryInterface)(void*, const TUID*, void**);
    uint32_t (*addRef)(void*);
    uint32_t (*release)(void*);
    uint32_t (*getParameterId)(void*);
    int32_t (*getPointCount)(void*);
    tresult (*getPoint)(void*, int32_t, int32_t*, double*);
} ParamQueueVtbl;

typedef struct {
    int32_t processMode;
    int32_t symbolicSampleSize;
    int32_t numSamples;
    int32_t numInputs;
    int32_t numOutputs;
    AudioBusBuffers* inputs;
    AudioBusBuffers* outputs;
    const ParamChangesVtbl** inputParameterChanges;
} ProcessData;

static MockPlugin g_plugin;

static tresult plug_qi(MockPlugin* p, const TUID* iid, void** o) {
    if (iid_eq(iid, IID_FUNKNOWN) || iid_eq(iid, IID_IPLUGINBASE) || iid_eq(iid, IID_ICOMPONENT)) {
        p->refs++; *o = &p->comp_vtbl; return kResultOk;
    }
    if (iid_eq(iid, IID_IAUDIOPROC)) {
        p->refs++; *o = &p->proc_vtbl; return kResultOk;
    }
    if (iid_eq(iid, IID_IEDITCTRL)) {
        p->refs++; *o = &p->edit_vtbl; return kResultOk;
    }
    *o = NULL; return kNoInterface;
}

static tresult comp_qi(void* t, const TUID* iid, void** o) { return plug_qi((MockPlugin*)t, iid, o); }
static uint32_t comp_add(void* t) { return ++((MockPlugin*)t)->refs; }
static uint32_t comp_rel(void* t) { return --((MockPlugin*)t)->refs; }
static tresult comp_init(void* t, void* ctx) { (void)t; (void)ctx; return kResultOk; }
static tresult comp_term(void* t) { (void)t; return kResultOk; }
static tresult comp_get_ctrl(void* t, TUID* cid) { (void)t; (void)cid; return -1; }
static int32_t comp_bus_cnt(void* t, int32_t m, int32_t d) { (void)t; (void)m; (void)d; return 1; }
static tresult comp_bus_info(void* t, int32_t m, int32_t d, int32_t i, void* b) { (void)t;(void)m;(void)d;(void)i;(void)b; return kResultOk; }
static tresult comp_set_active(void* t, uint8_t s) { ((MockPlugin*)t)->active = s ? 1 : 0; return kResultOk; }
static tresult comp_stub_ok(void* t, ...) { (void)t; return kResultOk; }

static MockPlugin* from_proc(void* t) { return (MockPlugin*)((uint8_t*)t - __builtin_offsetof(MockPlugin, proc_vtbl)); }
static tresult proc_qi(void* t, const TUID* iid, void** o) { return plug_qi(from_proc(t), iid, o); }
static uint32_t proc_add(void* t) { return ++from_proc(t)->refs; }
static uint32_t proc_rel(void* t) { return --from_proc(t)->refs; }
static tresult proc_setup(void* t, void* s) { (void)t; (void)s; return kResultOk; }
static tresult proc_set_processing(void* t, uint8_t s) { from_proc(t)->processing = s ? 1 : 0; return kResultOk; }
static tresult proc_process(void* t, ProcessData* data) {
    MockPlugin* p = from_proc(t);
    if (data->inputParameterChanges) {
        const ParamChangesVtbl** pc = data->inputParameterChanges;
        int32_t nq = (*pc)->getParameterCount(pc);
        for (int32_t i = 0; i < nq; i++) {
            const ParamQueueVtbl** q = (*pc)->getParameterData(pc, i);
            if (q && (*q)->getParameterId(q) == 0 && (*q)->getPointCount(q) > 0) {
                int32_t off = 0; double val = 1.0;
                if ((*q)->getPoint(q, 0, &off, &val) == kResultOk) {
                    p->gain = val;
                }
            }
        }
    }
    float g = (float)p->gain;
    for (int32_t ch = 0; ch < 2; ch++) {
        float* in = data->inputs[0].channelBuffers32[ch];
        float* out = data->outputs[0].channelBuffers32[ch];
        for (int32_t i = 0; i < data->numSamples; i++) {
            out[i] = in[i] * g;
        }
    }
    return kResultOk;
}
static uint32_t proc_zero(void* t) { (void)t; return 0; }

typedef struct {
    uint32_t id;
    uint16_t title[128];
    uint16_t shortTitle[128];
    uint16_t units[128];
    int32_t stepCount;
    double defaultNormalizedValue;
    int32_t unitId;
    int32_t flags;
} ParameterInfo;

static MockPlugin* from_edit(void* t) { return (MockPlugin*)((uint8_t*)t - __builtin_offsetof(MockPlugin, edit_vtbl)); }
static tresult edit_qi(void* t, const TUID* iid, void** o) { return plug_qi(from_edit(t), iid, o); }
static uint32_t edit_add(void* t) { return ++from_edit(t)->refs; }
static uint32_t edit_rel(void* t) { return --from_edit(t)->refs; }
static int32_t edit_param_cnt(void* t) { (void)t; return 1; }
static tresult edit_param_info(void* t, int32_t idx, ParameterInfo* info) {
    (void)t;
    if (idx != 0) return -1;
    memset(info, 0, sizeof(*info));
    info->id = 0;
    const char* name = "Gain";
    for (int i = 0; name[i]; i++) { info->title[i] = (uint16_t)name[i]; info->shortTitle[i] = (uint16_t)name[i]; }
    info->defaultNormalizedValue = 1.0;
    info->flags = 1;
    return kResultOk;
}
static double edit_get_norm(void* t, uint32_t id) { return (id == 0) ? from_edit(t)->gain : 0.0; }
static tresult edit_set_norm(void* t, uint32_t id, double val) { if (id == 0) from_edit(t)->gain = val; return kResultOk; }
static void* edit_create_view(void* t, const char* name) {
    (void)t;
    if (name && strcmp(name, "editor") == 0) { g_view.refs++; return &g_view; }
    return NULL;
}

static const void* COMP_VTBL[14] = {
    (void*)comp_qi, (void*)comp_add, (void*)comp_rel,
    (void*)comp_init, (void*)comp_term, (void*)comp_get_ctrl,
    (void*)comp_stub_ok, (void*)comp_bus_cnt, (void*)comp_bus_info,
    (void*)comp_stub_ok, (void*)comp_stub_ok, (void*)comp_set_active,
    (void*)comp_stub_ok, (void*)comp_stub_ok
};
static const void* PROC_VTBL[11] = {
    (void*)proc_qi, (void*)proc_add, (void*)proc_rel,
    (void*)comp_stub_ok, (void*)comp_stub_ok, (void*)comp_stub_ok,
    (void*)proc_zero, (void*)proc_setup, (void*)proc_set_processing,
    (void*)proc_process, (void*)proc_zero
};
static const void* EDIT_VTBL[18] = {
    (void*)edit_qi, (void*)edit_add, (void*)edit_rel,
    (void*)comp_init, (void*)comp_term, (void*)comp_stub_ok,
    (void*)comp_stub_ok, (void*)comp_stub_ok,
    (void*)edit_param_cnt, (void*)edit_param_info,
    (void*)comp_stub_ok, (void*)comp_stub_ok,
    (void*)edit_get_norm, (void*)edit_get_norm,
    (void*)edit_get_norm, (void*)edit_set_norm,
    (void*)comp_stub_ok, (void*)edit_create_view
};

typedef struct { char vendor[64]; char url[256]; char email[128]; int32_t flags; } PFactoryInfo;
typedef struct { TUID cid; int32_t cardinality; char category[32]; char name[64]; } PClassInfo;
typedef struct {
    TUID cid; int32_t cardinality; char category[32]; char name[64];
    uint32_t classFlags; char subCategories[128]; char vendor[64]; char version[64]; char sdkVersion[64];
} PClassInfo2;

typedef struct { const void* vtbl; uint32_t refs; } MockFactory;
static MockFactory g_factory;
static tresult fac_qi(void* t, const TUID* iid, void** o) {
    if (iid_eq(iid, IID_FUNKNOWN) || iid_eq(iid, IID_IFACTORY) || iid_eq(iid, IID_IFACTORY2)) {
        ((MockFactory*)t)->refs++; *o = t; return kResultOk;
    }
    *o = NULL; return kNoInterface;
}
static uint32_t fac_add(void* t) { return ++((MockFactory*)t)->refs; }
static uint32_t fac_rel(void* t) { return --((MockFactory*)t)->refs; }
static tresult fac_info(void* t, PFactoryInfo* info) {
    (void)t; memset(info, 0, sizeof(*info)); strcpy(info->vendor, "ALP Mock Vendor"); return kResultOk;
}
static int32_t fac_cnt(void* t) { (void)t; return 1; }
static tresult fac_class(void* t, int32_t idx, PClassInfo* info) {
    (void)t; if (idx != 0) return -1;
    memset(info, 0, sizeof(*info)); memcpy(&info->cid, CID_MOCK, 16);
    info->cardinality = 0x7fffffff; strcpy(info->category, "Audio Module Class");
    strcpy(info->name, "Mock VST3 Gain"); return kResultOk;
}
static tresult fac_create(void* t, const char* cid, const char* iid, void** o) {
    (void)t;
    if (cid && iid && iid_eq(cid, CID_MOCK) && (iid_eq(iid, IID_ICOMPONENT) || iid_eq(iid, IID_FUNKNOWN))) {
        g_plugin.comp_vtbl = COMP_VTBL;
        g_plugin.proc_vtbl = PROC_VTBL;
        g_plugin.edit_vtbl = EDIT_VTBL;
        g_plugin.refs = 1;
        g_plugin.gain = 1.0;
        *o = &g_plugin;
        return kResultOk;
    }
    *o = NULL; return kNoInterface;
}
static tresult fac_class2(void* t, int32_t idx, PClassInfo2* info) {
    (void)t; if (idx != 0) return -1;
    memset(info, 0, sizeof(*info)); memcpy(&info->cid, CID_MOCK, 16);
    info->cardinality = 0x7fffffff; strcpy(info->category, "Audio Module Class");
    strcpy(info->name, "Mock VST3 Gain"); strcpy(info->subCategories, "Fx");
    strcpy(info->vendor, "ALP Mock Vendor"); strcpy(info->version, "2.1.0");
    return kResultOk;
}
static const void* FAC_VTBL[9] = {
    (void*)fac_qi, (void*)fac_add, (void*)fac_rel,
    (void*)fac_info, (void*)fac_cnt, (void*)fac_class,
    (void*)fac_create, (void*)fac_class2, (void*)comp_stub_ok
};

bool ModuleEntry(void* handle) { (void)handle; g_module_refs++; return true; }
bool ModuleExit(void) { g_module_refs--; return true; }
void* GetPluginFactory(void) { g_factory.vtbl = FAC_VTBL; g_factory.refs = 1; return &g_factory; }
uintptr_t alp_test_last_parent_xid(void) { return g_last_parent_xid; }
int alp_test_view_attached(void) { return g_view_attached; }
"#;

#[test]
fn instantiates_processes_automates_and_embeds_real_vst3_plugin() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bundle = dir.path().join("MockGain.vst3");
    let binary_dir = bundle.join("Contents/x86_64-linux");
    std::fs::create_dir_all(&binary_dir).expect("bundle dirs");
    let c_path = dir.path().join("mock_gain.c");
    let so_path = binary_dir.join("MockGain.so");
    std::fs::write(&c_path, MOCK_VST3_C).expect("write mock c");

    let status = Command::new("cc")
        .args([
            "-shared",
            "-fPIC",
            "-O2",
            c_path.to_str().unwrap(),
            "-o",
            so_path.to_str().unwrap(),
        ])
        .status()
        .expect("invoke cc");
    assert!(status.success(), "cc must compile mock VST3 library");

    let desc = scan_vst3_bundle(&bundle).expect("scan mock vst3");
    assert_eq!(desc.format, PluginFormat::Vst3);
    assert_eq!(desc.name, "Mock VST3 Gain");
    assert_eq!(desc.vendor, "ALP Mock Vendor");
    assert_eq!(desc.version, "2.1.0");

    let mut plugin = ActivePlugin::instantiate(&desc, 48_000).expect("instantiate vst3");
    assert_eq!(plugin.parameters().len(), 1);
    assert_eq!(plugin.parameters()[0].symbol, "gain");

    let ctx = ProcessContext {
        sample_rate: 48_000,
        num_frames: 4,
        frame_position: 0,
        is_playing: true,
    };
    let in_l = [1.0f32, 0.5, -0.5, -1.0];
    let in_r = [0.8f32, 0.4, -0.4, -0.8];
    let mut out_l = [0.0f32; 4];
    let mut out_r = [0.0f32; 4];

    plugin
        .process(&in_l, &in_r, &mut out_l, &mut out_r, &ctx)
        .expect("process initial gain 1.0");
    assert!((out_l[0] - 1.0).abs() < 1e-5);
    assert!((out_r[0] - 0.8).abs() < 1e-5);

    plugin.set_parameter("gain", 0.25).expect("set gain 0.25");
    plugin
        .process(&in_l, &in_r, &mut out_l, &mut out_r, &ctx)
        .expect("process updated gain 0.25");
    assert!((out_l[0] - 0.25).abs() < 1e-5);
    assert!((out_r[0] - 0.2).abs() < 1e-5);

    let editor_size = plugin
        .open_editor_x11(0x1234_5678)
        .expect("attach X11 editor");
    assert_eq!(editor_size, (720, 420));
    plugin.close_editor_x11();
}
