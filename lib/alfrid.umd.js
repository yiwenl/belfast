(function (global, factory) {
  typeof exports === 'object' && typeof module !== 'undefined' ? factory(exports) :
  typeof define === 'function' && define.amd ? define(['exports'], factory) :
  (global = global || self, factory(global.alfrid = {}));
}(this, (function (exports) { 'use strict';

  /**
   * Common utilities
   * @module glMatrix
   */
  var ARRAY_TYPE = typeof Float32Array !== 'undefined' ? Float32Array : Array;
  if (!Math.hypot) Math.hypot = function () {
    var y = 0,
        i = arguments.length;

    while (i--) {
      y += arguments[i] * arguments[i];
    }

    return Math.sqrt(y);
  };

  /**
   * 3x3 Matrix
   * @module mat3
   */

  /**
   * Creates a new identity mat3
   *
   * @returns {mat3} a new 3x3 matrix
   */

  function create() {
    var out = new ARRAY_TYPE(9);

    if (ARRAY_TYPE != Float32Array) {
      out[1] = 0;
      out[2] = 0;
      out[3] = 0;
      out[5] = 0;
      out[6] = 0;
      out[7] = 0;
    }

    out[0] = 1;
    out[4] = 1;
    out[8] = 1;
    return out;
  }

  /**
   * 4x4 Matrix<br>Format: column-major, when typed out it looks like row-major<br>The matrices are being post multiplied.
   * @module mat4
   */

  /**
   * Creates a new identity mat4
   *
   * @returns {mat4} a new 4x4 matrix
   */

  function create$1() {
    var out = new ARRAY_TYPE(16);

    if (ARRAY_TYPE != Float32Array) {
      out[1] = 0;
      out[2] = 0;
      out[3] = 0;
      out[4] = 0;
      out[6] = 0;
      out[7] = 0;
      out[8] = 0;
      out[9] = 0;
      out[11] = 0;
      out[12] = 0;
      out[13] = 0;
      out[14] = 0;
    }

    out[0] = 1;
    out[5] = 1;
    out[10] = 1;
    out[15] = 1;
    return out;
  }

  const checkWebGL2 = () => {
    const canvas = document.createElement("canvas");
    const ctx =
      canvas.getContext("experimental-webgl2") || canvas.getContext("webgl2");
    return !!ctx;
  };

  const mobileCheck = () => {
    let isMobile = false;
    if (
      /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(
        navigator.userAgent
      )
    ) {
      isMobile = true;
    }

    return isMobile;
  };

  const isMobile = mobileCheck();

  const getWebGLContext = (mCanvas, mParams, mWebGL2 = false) => {
    if (mWebGL2) {
      return (
        mCanvas.getContext("webgl2", mParams) ||
        mCanvas.getContext("experimental-webgl2", mParams)
      );
    } else {
      return (
        mCanvas.getContext("webgl", mParams) ||
        mCanvas.getContext("experimental-webgl", mParams)
      );
    }
  };

  const checkViewport = (viewport, x, y, w, h) => {
    let hasChanged = false;
    if (x !== viewport[0]) {
      hasChanged = true;
    }
    if (y !== viewport[1]) {
      hasChanged = true;
    }
    if (w !== viewport[2]) {
      hasChanged = true;
    }
    if (h !== viewport[3]) {
      hasChanged = true;
    }
    return hasChanged;
  };

  // WebglConst.js

  // stolen there https://github.com/mattdesl/gl-constants thanks @mattdesl ^^

  var WebglConst = {
    ACTIVE_ATTRIBUTES: 35721,
    ACTIVE_ATTRIBUTE_MAX_LENGTH: 35722,
    ACTIVE_TEXTURE: 34016,
    ACTIVE_UNIFORMS: 35718,
    ACTIVE_UNIFORM_MAX_LENGTH: 35719,
    ALIASED_LINE_WIDTH_RANGE: 33902,
    ALIASED_POINT_SIZE_RANGE: 33901,
    ALPHA: 6406,
    ALPHA_BITS: 3413,
    ALWAYS: 519,
    ARRAY_BUFFER: 34962,
    ARRAY_BUFFER_BINDING: 34964,
    ATTACHED_SHADERS: 35717,
    BACK: 1029,
    BLEND: 3042,
    BLEND_COLOR: 32773,
    BLEND_DST_ALPHA: 32970,
    BLEND_DST_RGB: 32968,
    BLEND_EQUATION: 32777,
    BLEND_EQUATION_ALPHA: 34877,
    BLEND_EQUATION_RGB: 32777,
    BLEND_SRC_ALPHA: 32971,
    BLEND_SRC_RGB: 32969,
    BLUE_BITS: 3412,
    BOOL: 35670,
    BOOL_VEC2: 35671,
    BOOL_VEC3: 35672,
    BOOL_VEC4: 35673,
    BROWSER_DEFAULT_WEBGL: 37444,
    BUFFER_SIZE: 34660,
    BUFFER_USAGE: 34661,
    BYTE: 5120,
    CCW: 2305,
    CLAMP_TO_EDGE: 33071,
    COLOR_ATTACHMENT0: 36064,
    COLOR_BUFFER_BIT: 16384,
    COLOR_CLEAR_VALUE: 3106,
    COLOR_WRITEMASK: 3107,
    COMPILE_STATUS: 35713,
    COMPRESSED_TEXTURE_FORMATS: 34467,
    CONSTANT_ALPHA: 32771,
    CONSTANT_COLOR: 32769,
    CONTEXT_LOST_WEBGL: 37442,
    CULL_FACE: 2884,
    CULL_FACE_MODE: 2885,
    CURRENT_PROGRAM: 35725,
    CURRENT_VERTEX_ATTRIB: 34342,
    CW: 2304,
    DECR: 7683,
    DECR_WRAP: 34056,
    DELETE_STATUS: 35712,
    DEPTH_ATTACHMENT: 36096,
    DEPTH_BITS: 3414,
    DEPTH_BUFFER_BIT: 256,
    DEPTH_CLEAR_VALUE: 2931,
    DEPTH_COMPONENT: 6402,
    RED: 6403,
    DEPTH_COMPONENT16: 33189,
    DEPTH_FUNC: 2932,
    DEPTH_RANGE: 2928,
    DEPTH_STENCIL: 34041,
    DEPTH_STENCIL_ATTACHMENT: 33306,
    DEPTH_TEST: 2929,
    DEPTH_WRITEMASK: 2930,
    DITHER: 3024,
    DONT_CARE: 4352,
    DST_ALPHA: 772,
    DST_COLOR: 774,
    DYNAMIC_DRAW: 35048,
    ELEMENT_ARRAY_BUFFER: 34963,
    ELEMENT_ARRAY_BUFFER_BINDING: 34965,
    EQUAL: 514,
    FASTEST: 4353,
    FLOAT: 5126,
    FLOAT_MAT2: 35674,
    FLOAT_MAT3: 35675,
    FLOAT_MAT4: 35676,
    FLOAT_VEC2: 35664,
    FLOAT_VEC3: 35665,
    FLOAT_VEC4: 35666,
    FRAGMENT_SHADER: 35632,
    FRAMEBUFFER: 36160,
    FRAMEBUFFER_ATTACHMENT_OBJECT_NAME: 36049,
    FRAMEBUFFER_ATTACHMENT_OBJECT_TYPE: 36048,
    FRAMEBUFFER_ATTACHMENT_TEXTURE_CUBE_MAP_FACE: 36051,
    FRAMEBUFFER_ATTACHMENT_TEXTURE_LEVEL: 36050,
    FRAMEBUFFER_BINDING: 36006,
    FRAMEBUFFER_COMPLETE: 36053,
    FRAMEBUFFER_INCOMPLETE_ATTACHMENT: 36054,
    FRAMEBUFFER_INCOMPLETE_DIMENSIONS: 36057,
    FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT: 36055,
    FRAMEBUFFER_UNSUPPORTED: 36061,
    FRONT: 1028,
    FRONT_AND_BACK: 1032,
    FRONT_FACE: 2886,
    FUNC_ADD: 32774,
    FUNC_REVERSE_SUBTRACT: 32779,
    FUNC_SUBTRACT: 32778,
    GENERATE_MIPMAP_HINT: 33170,
    GEQUAL: 518,
    GREATER: 516,
    GREEN_BITS: 3411,
    HIGH_FLOAT: 36338,
    HIGH_INT: 36341,
    INCR: 7682,
    INCR_WRAP: 34055,
    INFO_LOG_LENGTH: 35716,
    INT: 5124,
    INT_VEC2: 35667,
    INT_VEC3: 35668,
    INT_VEC4: 35669,
    INVALID_ENUM: 1280,
    INVALID_FRAMEBUFFER_OPERATION: 1286,
    INVALID_OPERATION: 1282,
    INVALID_VALUE: 1281,
    INVERT: 5386,
    KEEP: 7680,
    LEQUAL: 515,
    LESS: 513,
    LINEAR: 9729,
    LINEAR_MIPMAP_LINEAR: 9987,
    LINEAR_MIPMAP_NEAREST: 9985,
    LINES: 1,
    LINE_LOOP: 2,
    LINE_STRIP: 3,
    LINE_WIDTH: 2849,
    LINK_STATUS: 35714,
    LOW_FLOAT: 36336,
    LOW_INT: 36339,
    LUMINANCE: 6409,
    LUMINANCE_ALPHA: 6410,
    MAX_COMBINED_TEXTURE_IMAGE_UNITS: 35661,
    MAX_CUBE_MAP_TEXTURE_SIZE: 34076,
    MAX_FRAGMENT_UNIFORM_VECTORS: 36349,
    MAX_RENDERBUFFER_SIZE: 34024,
    MAX_TEXTURE_IMAGE_UNITS: 34930,
    MAX_TEXTURE_SIZE: 3379,
    MAX_VARYING_VECTORS: 36348,
    MAX_VERTEX_ATTRIBS: 34921,
    MAX_VERTEX_TEXTURE_IMAGE_UNITS: 35660,
    MAX_VERTEX_UNIFORM_VECTORS: 36347,
    MAX_VIEWPORT_DIMS: 3386,
    MEDIUM_FLOAT: 36337,
    MEDIUM_INT: 36340,
    MIRRORED_REPEAT: 33648,
    NEAREST: 9728,
    NEAREST_MIPMAP_LINEAR: 9986,
    NEAREST_MIPMAP_NEAREST: 9984,
    NEVER: 512,
    NICEST: 4354,
    NONE: 0,
    NOTEQUAL: 517,
    NO_ERROR: 0,
    NUM_COMPRESSED_TEXTURE_FORMATS: 34466,
    ONE: 1,
    ONE_MINUS_CONSTANT_ALPHA: 32772,
    ONE_MINUS_CONSTANT_COLOR: 32770,
    ONE_MINUS_DST_ALPHA: 773,
    ONE_MINUS_DST_COLOR: 775,
    ONE_MINUS_SRC_ALPHA: 771,
    ONE_MINUS_SRC_COLOR: 769,
    OUT_OF_MEMORY: 1285,
    PACK_ALIGNMENT: 3333,
    POINTS: 0,
    POLYGON_OFFSET_FACTOR: 32824,
    POLYGON_OFFSET_FILL: 32823,
    POLYGON_OFFSET_UNITS: 10752,
    RED_BITS: 3410,
    RENDERBUFFER: 36161,
    RENDERBUFFER_ALPHA_SIZE: 36179,
    RENDERBUFFER_BINDING: 36007,
    RENDERBUFFER_BLUE_SIZE: 36178,
    RENDERBUFFER_DEPTH_SIZE: 36180,
    RENDERBUFFER_GREEN_SIZE: 36177,
    RENDERBUFFER_HEIGHT: 36163,
    RENDERBUFFER_INTERNAL_FORMAT: 36164,
    RENDERBUFFER_RED_SIZE: 36176,
    RENDERBUFFER_STENCIL_SIZE: 36181,
    RENDERBUFFER_WIDTH: 36162,
    RENDERER: 7937,
    REPEAT: 10497,
    REPLACE: 7681,
    RGB: 6407,
    RGB5_A1: 32855,
    RGB565: 36194,
    RGBA: 6408,
    RGBA4: 32854,
    SAMPLER_2D: 35678,
    SAMPLER_CUBE: 35680,
    SAMPLES: 32937,
    SAMPLE_ALPHA_TO_COVERAGE: 32926,
    SAMPLE_BUFFERS: 32936,
    SAMPLE_COVERAGE: 32928,
    SAMPLE_COVERAGE_INVERT: 32939,
    SAMPLE_COVERAGE_VALUE: 32938,
    SCISSOR_BOX: 3088,
    SCISSOR_TEST: 3089,
    SHADER_COMPILER: 36346,
    SHADER_SOURCE_LENGTH: 35720,
    SHADER_TYPE: 35663,
    SHADING_LANGUAGE_VERSION: 35724,
    SHORT: 5122,
    SRC_ALPHA: 770,
    SRC_ALPHA_SATURATE: 776,
    SRC_COLOR: 768,
    STATIC_DRAW: 35044,
    STENCIL_ATTACHMENT: 36128,
    STENCIL_BACK_FAIL: 34817,
    STENCIL_BACK_FUNC: 34816,
    STENCIL_BACK_PASS_DEPTH_FAIL: 34818,
    STENCIL_BACK_PASS_DEPTH_PASS: 34819,
    STENCIL_BACK_REF: 36003,
    STENCIL_BACK_VALUE_MASK: 36004,
    STENCIL_BACK_WRITEMASK: 36005,
    STENCIL_BITS: 3415,
    STENCIL_BUFFER_BIT: 1024,
    STENCIL_CLEAR_VALUE: 2961,
    STENCIL_FAIL: 2964,
    STENCIL_FUNC: 2962,
    STENCIL_INDEX: 6401,
    STENCIL_INDEX8: 36168,
    STENCIL_PASS_DEPTH_FAIL: 2965,
    STENCIL_PASS_DEPTH_PASS: 2966,
    STENCIL_REF: 2967,
    STENCIL_TEST: 2960,
    STENCIL_VALUE_MASK: 2963,
    STENCIL_WRITEMASK: 2968,
    STREAM_DRAW: 35040,
    SUBPIXEL_BITS: 3408,
    TEXTURE: 5890,
    TEXTURE0: 33984,
    TEXTURE1: 33985,
    TEXTURE2: 33986,
    TEXTURE3: 33987,
    TEXTURE4: 33988,
    TEXTURE5: 33989,
    TEXTURE6: 33990,
    TEXTURE7: 33991,
    TEXTURE8: 33992,
    TEXTURE9: 33993,
    TEXTURE10: 33994,
    TEXTURE11: 33995,
    TEXTURE12: 33996,
    TEXTURE13: 33997,
    TEXTURE14: 33998,
    TEXTURE15: 33999,
    TEXTURE16: 34000,
    TEXTURE17: 34001,
    TEXTURE18: 34002,
    TEXTURE19: 34003,
    TEXTURE20: 34004,
    TEXTURE21: 34005,
    TEXTURE22: 34006,
    TEXTURE23: 34007,
    TEXTURE24: 34008,
    TEXTURE25: 34009,
    TEXTURE26: 34010,
    TEXTURE27: 34011,
    TEXTURE28: 34012,
    TEXTURE29: 34013,
    TEXTURE30: 34014,
    TEXTURE31: 34015,
    TEXTURE_2D: 3553,
    TEXTURE_BINDING_2D: 32873,
    TEXTURE_BINDING_CUBE_MAP: 34068,
    TEXTURE_CUBE_MAP: 34067,
    TEXTURE_CUBE_MAP_NEGATIVE_X: 34070,
    TEXTURE_CUBE_MAP_NEGATIVE_Y: 34072,
    TEXTURE_CUBE_MAP_NEGATIVE_Z: 34074,
    TEXTURE_CUBE_MAP_POSITIVE_X: 34069,
    TEXTURE_CUBE_MAP_POSITIVE_Y: 34071,
    TEXTURE_CUBE_MAP_POSITIVE_Z: 34073,
    TEXTURE_MAG_FILTER: 10240,
    TEXTURE_MIN_FILTER: 10241,
    TEXTURE_WRAP_S: 10242,
    TEXTURE_WRAP_T: 10243,
    TRIANGLES: 4,
    TRIANGLE_FAN: 6,
    TRIANGLE_STRIP: 5,
    UNPACK_ALIGNMENT: 3317,
    UNPACK_COLORSPACE_CONVERSION_WEBGL: 37443,
    UNPACK_FLIP_Y_WEBGL: 37440,
    UNPACK_PREMULTIPLY_ALPHA_WEBGL: 37441,
    UNSIGNED_BYTE: 5121,
    UNSIGNED_INT: 5125,
    UNSIGNED_SHORT: 5123,
    UNSIGNED_SHORT_4_4_4_4: 32819,
    UNSIGNED_SHORT_5_5_5_1: 32820,
    UNSIGNED_SHORT_5_6_5: 33635,
    VALIDATE_STATUS: 35715,
    VENDOR: 7936,
    VERSION: 7938,
    VERTEX_ATTRIB_ARRAY_BUFFER_BINDING: 34975,
    VERTEX_ATTRIB_ARRAY_ENABLED: 34338,
    VERTEX_ATTRIB_ARRAY_NORMALIZED: 34922,
    VERTEX_ATTRIB_ARRAY_POINTER: 34373,
    VERTEX_ATTRIB_ARRAY_SIZE: 34339,
    VERTEX_ATTRIB_ARRAY_STRIDE: 34340,
    VERTEX_ATTRIB_ARRAY_TYPE: 34341,
    VERTEX_SHADER: 35633,
    VIEWPORT: 2978,
    ZERO: 0,
    R8: 33321,
  };

  // stolen there https://github.com/mattdesl/gl-constants thanks @mattdesl ^^
  var WebglNumber = {
    0: "NONE",
    1: "ONE",
    2: "LINE_LOOP",
    3: "LINE_STRIP",
    4: "TRIANGLES",
    5: "TRIANGLE_STRIP",
    6: "TRIANGLE_FAN",
    256: "DEPTH_BUFFER_BIT",
    512: "NEVER",
    513: "LESS",
    514: "EQUAL",
    515: "LEQUAL",
    516: "GREATER",
    517: "NOTEQUAL",
    518: "GEQUAL",
    519: "ALWAYS",
    768: "SRC_COLOR",
    769: "ONE_MINUS_SRC_COLOR",
    770: "SRC_ALPHA",
    771: "ONE_MINUS_SRC_ALPHA",
    772: "DST_ALPHA",
    773: "ONE_MINUS_DST_ALPHA",
    774: "DST_COLOR",
    775: "ONE_MINUS_DST_COLOR",
    776: "SRC_ALPHA_SATURATE",
    1024: "STENCIL_BUFFER_BIT",
    1028: "FRONT",
    1029: "BACK",
    1032: "FRONT_AND_BACK",
    1280: "INVALID_ENUM",
    1281: "INVALID_VALUE",
    1282: "INVALID_OPERATION",
    1285: "OUT_OF_MEMORY",
    1286: "INVALID_FRAMEBUFFER_OPERATION",
    2304: "CW",
    2305: "CCW",
    2849: "LINE_WIDTH",
    2884: "CULL_FACE",
    2885: "CULL_FACE_MODE",
    2886: "FRONT_FACE",
    2928: "DEPTH_RANGE",
    2929: "DEPTH_TEST",
    2930: "DEPTH_WRITEMASK",
    2931: "DEPTH_CLEAR_VALUE",
    2932: "DEPTH_FUNC",
    2960: "STENCIL_TEST",
    2961: "STENCIL_CLEAR_VALUE",
    2962: "STENCIL_FUNC",
    2963: "STENCIL_VALUE_MASK",
    2964: "STENCIL_FAIL",
    2965: "STENCIL_PASS_DEPTH_FAIL",
    2966: "STENCIL_PASS_DEPTH_PASS",
    2967: "STENCIL_REF",
    2968: "STENCIL_WRITEMASK",
    2978: "VIEWPORT",
    3024: "DITHER",
    3042: "BLEND",
    3088: "SCISSOR_BOX",
    3089: "SCISSOR_TEST",
    3106: "COLOR_CLEAR_VALUE",
    3107: "COLOR_WRITEMASK",
    3317: "UNPACK_ALIGNMENT",
    3333: "PACK_ALIGNMENT",
    3379: "MAX_TEXTURE_SIZE",
    3386: "MAX_VIEWPORT_DIMS",
    3408: "SUBPIXEL_BITS",
    3410: "RED_BITS",
    3411: "GREEN_BITS",
    3412: "BLUE_BITS",
    3413: "ALPHA_BITS",
    3414: "DEPTH_BITS",
    3415: "STENCIL_BITS",
    3553: "TEXTURE_2D",
    4352: "DONT_CARE",
    4353: "FASTEST",
    4354: "NICEST",
    5120: "BYTE",
    5121: "UNSIGNED_BYTE",
    5122: "SHORT",
    5123: "UNSIGNED_SHORT",
    5124: "INT",
    5125: "UNSIGNED_INT",
    5126: "FLOAT",
    5386: "INVERT",
    5890: "TEXTURE",
    6401: "STENCIL_INDEX",
    6402: "DEPTH_COMPONENT",
    6403: "RED",
    6406: "ALPHA",
    6407: "RGB",
    6408: "RGBA",
    6409: "LUMINANCE",
    6410: "LUMINANCE_ALPHA",
    7680: "KEEP",
    7681: "REPLACE",
    7682: "INCR",
    7683: "DECR",
    7936: "VENDOR",
    7937: "RENDERER",
    7938: "VERSION",
    9728: "NEAREST",
    9729: "LINEAR",
    9984: "NEAREST_MIPMAP_NEAREST",
    9985: "LINEAR_MIPMAP_NEAREST",
    9986: "NEAREST_MIPMAP_LINEAR",
    9987: "LINEAR_MIPMAP_LINEAR",
    10240: "TEXTURE_MAG_FILTER",
    10241: "TEXTURE_MIN_FILTER",
    10242: "TEXTURE_WRAP_S",
    10243: "TEXTURE_WRAP_T",
    10497: "REPEAT",
    10752: "POLYGON_OFFSET_UNITS",
    16384: "COLOR_BUFFER_BIT",
    32769: "CONSTANT_COLOR",
    32770: "ONE_MINUS_CONSTANT_COLOR",
    32771: "CONSTANT_ALPHA",
    32772: "ONE_MINUS_CONSTANT_ALPHA",
    32773: "BLEND_COLOR",
    32774: "FUNC_ADD",
    32777: "BLEND_EQUATION_RGB",
    32778: "FUNC_SUBTRACT",
    32779: "FUNC_REVERSE_SUBTRACT",
    32819: "UNSIGNED_SHORT_4_4_4_4",
    32820: "UNSIGNED_SHORT_5_5_5_1",
    32823: "POLYGON_OFFSET_FILL",
    32824: "POLYGON_OFFSET_FACTOR",
    32854: "RGBA4",
    32855: "RGB5_A1",
    32873: "TEXTURE_BINDING_2D",
    32926: "SAMPLE_ALPHA_TO_COVERAGE",
    32928: "SAMPLE_COVERAGE",
    32936: "SAMPLE_BUFFERS",
    32937: "SAMPLES",
    32938: "SAMPLE_COVERAGE_VALUE",
    32939: "SAMPLE_COVERAGE_INVERT",
    32968: "BLEND_DST_RGB",
    32969: "BLEND_SRC_RGB",
    32970: "BLEND_DST_ALPHA",
    32971: "BLEND_SRC_ALPHA",
    33071: "CLAMP_TO_EDGE",
    33170: "GENERATE_MIPMAP_HINT",
    33189: "DEPTH_COMPONENT16",
    33306: "DEPTH_STENCIL_ATTACHMENT",
    33321: "R8",
    33635: "UNSIGNED_SHORT_5_6_5",
    33648: "MIRRORED_REPEAT",
    33901: "ALIASED_POINT_SIZE_RANGE",
    33902: "ALIASED_LINE_WIDTH_RANGE",
    33984: "TEXTURE0",
    33985: "TEXTURE1",
    33986: "TEXTURE2",
    33987: "TEXTURE3",
    33988: "TEXTURE4",
    33989: "TEXTURE5",
    33990: "TEXTURE6",
    33991: "TEXTURE7",
    33992: "TEXTURE8",
    33993: "TEXTURE9",
    33994: "TEXTURE10",
    33995: "TEXTURE11",
    33996: "TEXTURE12",
    33997: "TEXTURE13",
    33998: "TEXTURE14",
    33999: "TEXTURE15",
    34000: "TEXTURE16",
    34001: "TEXTURE17",
    34002: "TEXTURE18",
    34003: "TEXTURE19",
    34004: "TEXTURE20",
    34005: "TEXTURE21",
    34006: "TEXTURE22",
    34007: "TEXTURE23",
    34008: "TEXTURE24",
    34009: "TEXTURE25",
    34010: "TEXTURE26",
    34011: "TEXTURE27",
    34012: "TEXTURE28",
    34013: "TEXTURE29",
    34014: "TEXTURE30",
    34015: "TEXTURE31",
    34016: "ACTIVE_TEXTURE",
    34024: "MAX_RENDERBUFFER_SIZE",
    34041: "DEPTH_STENCIL",
    34055: "INCR_WRAP",
    34056: "DECR_WRAP",
    34067: "TEXTURE_CUBE_MAP",
    34068: "TEXTURE_BINDING_CUBE_MAP",
    34069: "TEXTURE_CUBE_MAP_POSITIVE_X",
    34070: "TEXTURE_CUBE_MAP_NEGATIVE_X",
    34071: "TEXTURE_CUBE_MAP_POSITIVE_Y",
    34072: "TEXTURE_CUBE_MAP_NEGATIVE_Y",
    34073: "TEXTURE_CUBE_MAP_POSITIVE_Z",
    34074: "TEXTURE_CUBE_MAP_NEGATIVE_Z",
    34076: "MAX_CUBE_MAP_TEXTURE_SIZE",
    34338: "VERTEX_ATTRIB_ARRAY_ENABLED",
    34339: "VERTEX_ATTRIB_ARRAY_SIZE",
    34340: "VERTEX_ATTRIB_ARRAY_STRIDE",
    34341: "VERTEX_ATTRIB_ARRAY_TYPE",
    34342: "CURRENT_VERTEX_ATTRIB",
    34373: "VERTEX_ATTRIB_ARRAY_POINTER",
    34466: "NUM_COMPRESSED_TEXTURE_FORMATS",
    34467: "COMPRESSED_TEXTURE_FORMATS",
    34660: "BUFFER_SIZE",
    34661: "BUFFER_USAGE",
    34816: "STENCIL_BACK_FUNC",
    34817: "STENCIL_BACK_FAIL",
    34818: "STENCIL_BACK_PASS_DEPTH_FAIL",
    34819: "STENCIL_BACK_PASS_DEPTH_PASS",
    34877: "BLEND_EQUATION_ALPHA",
    34921: "MAX_VERTEX_ATTRIBS",
    34922: "VERTEX_ATTRIB_ARRAY_NORMALIZED",
    34930: "MAX_TEXTURE_IMAGE_UNITS",
    34962: "ARRAY_BUFFER",
    34963: "ELEMENT_ARRAY_BUFFER",
    34964: "ARRAY_BUFFER_BINDING",
    34965: "ELEMENT_ARRAY_BUFFER_BINDING",
    34975: "VERTEX_ATTRIB_ARRAY_BUFFER_BINDING",
    35040: "STREAM_DRAW",
    35044: "STATIC_DRAW",
    35048: "DYNAMIC_DRAW",
    35632: "FRAGMENT_SHADER",
    35633: "VERTEX_SHADER",
    35660: "MAX_VERTEX_TEXTURE_IMAGE_UNITS",
    35661: "MAX_COMBINED_TEXTURE_IMAGE_UNITS",
    35663: "SHADER_TYPE",
    35664: "FLOAT_VEC2",
    35665: "FLOAT_VEC3",
    35666: "FLOAT_VEC4",
    35667: "INT_VEC2",
    35668: "INT_VEC3",
    35669: "INT_VEC4",
    35670: "BOOL",
    35671: "BOOL_VEC2",
    35672: "BOOL_VEC3",
    35673: "BOOL_VEC4",
    35674: "FLOAT_MAT2",
    35675: "FLOAT_MAT3",
    35676: "FLOAT_MAT4",
    35678: "SAMPLER_2D",
    35680: "SAMPLER_CUBE",
    35712: "DELETE_STATUS",
    35713: "COMPILE_STATUS",
    35714: "LINK_STATUS",
    35715: "VALIDATE_STATUS",
    35716: "INFO_LOG_LENGTH",
    35717: "ATTACHED_SHADERS",
    35718: "ACTIVE_UNIFORMS",
    35719: "ACTIVE_UNIFORM_MAX_LENGTH",
    35720: "SHADER_SOURCE_LENGTH",
    35721: "ACTIVE_ATTRIBUTES",
    35722: "ACTIVE_ATTRIBUTE_MAX_LENGTH",
    35724: "SHADING_LANGUAGE_VERSION",
    35725: "CURRENT_PROGRAM",
    36003: "STENCIL_BACK_REF",
    36004: "STENCIL_BACK_VALUE_MASK",
    36005: "STENCIL_BACK_WRITEMASK",
    36006: "FRAMEBUFFER_BINDING",
    36007: "RENDERBUFFER_BINDING",
    36048: "FRAMEBUFFER_ATTACHMENT_OBJECT_TYPE",
    36049: "FRAMEBUFFER_ATTACHMENT_OBJECT_NAME",
    36050: "FRAMEBUFFER_ATTACHMENT_TEXTURE_LEVEL",
    36051: "FRAMEBUFFER_ATTACHMENT_TEXTURE_CUBE_MAP_FACE",
    36053: "FRAMEBUFFER_COMPLETE",
    36054: "FRAMEBUFFER_INCOMPLETE_ATTACHMENT",
    36055: "FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT",
    36057: "FRAMEBUFFER_INCOMPLETE_DIMENSIONS",
    36061: "FRAMEBUFFER_UNSUPPORTED",
    36064: "COLOR_ATTACHMENT0",
    36096: "DEPTH_ATTACHMENT",
    36128: "STENCIL_ATTACHMENT",
    36160: "FRAMEBUFFER",
    36161: "RENDERBUFFER",
    36162: "RENDERBUFFER_WIDTH",
    36163: "RENDERBUFFER_HEIGHT",
    36164: "RENDERBUFFER_INTERNAL_FORMAT",
    36168: "STENCIL_INDEX8",
    36176: "RENDERBUFFER_RED_SIZE",
    36177: "RENDERBUFFER_GREEN_SIZE",
    36178: "RENDERBUFFER_BLUE_SIZE",
    36179: "RENDERBUFFER_ALPHA_SIZE",
    36180: "RENDERBUFFER_DEPTH_SIZE",
    36181: "RENDERBUFFER_STENCIL_SIZE",
    36194: "RGB565",
    36336: "LOW_FLOAT",
    36337: "MEDIUM_FLOAT",
    36338: "HIGH_FLOAT",
    36339: "LOW_INT",
    36340: "MEDIUM_INT",
    36341: "HIGH_INT",
    36346: "SHADER_COMPILER",
    36347: "MAX_VERTEX_UNIFORM_VECTORS",
    36348: "MAX_VARYING_VECTORS",
    36349: "MAX_FRAGMENT_UNIFORM_VECTORS",
    37440: "UNPACK_FLIP_Y_WEBGL",
    37441: "UNPACK_PREMULTIPLY_ALPHA_WEBGL",
    37442: "CONTEXT_LOST_WEBGL",
    37443: "UNPACK_COLORSPACE_CONVERSION_WEBGL",
    37444: "BROWSER_DEFAULT_WEBGL",
  };

  // exposeAttributes.js

  const exposeAttributes = (GL) => {
    for (const s in WebglConst) {
      if (!GL[s]) {
        GL[s] = WebglConst[s];
      } else {
        if (s !== "FLOAT") console.log("already exist : ", s);
      }
    }

    if (GL.webgl2) {
      const check = /^[^a-z]*$/;
      for (const s in GL.gl) {
        if (check.test(s) && s.indexOf("FLOAT") === -1) {
          GL[s] = GL.gl[s];
          WebglConst[s] = GL.gl[s];
          WebglNumber[GL[s]] = s;
        }
      }
    }
  };

  // VertexArrayObject.js

  function getAndApplyExtension(gl, name) {
    const ext = gl.getExtension(name);
    if (!ext) {
      return false;
    }
    const suffix = name.split("_")[0];
    const suffixRE = new RegExp(`${suffix}$`);

    for (const key in ext) {
      const val = ext[key];
      if (typeof val === "function") {
        const unsuffixedKey = key.replace(suffixRE, "");
        if (key.substring) {
          gl[unsuffixedKey] = ext[key].bind(ext);
          // console.log('Replacing :', key, '=>', unsuffixedKey);
        }
      }
    }

    return true;
  }

  // ExtensionsList.js

  var ExtensionsList = [
    "EXT_shader_texture_lod",
    "EXT_sRGB",
    "EXT_frag_depth",
    "OES_texture_float",
    "OES_texture_half_float",
    "OES_texture_float_linear",
    "OES_texture_half_float_linear",
    "OES_standard_derivatives",
    "WEBGL_depth_texture",
    "EXT_texture_filter_anisotropic",
    "OES_vertex_array_object",
    "ANGLE_instanced_arrays",
    "WEBGL_draw_buffers",
  ];

  /*
  object-assign
  (c) Sindre Sorhus
  @license MIT
  */
  /* eslint-disable no-unused-vars */
  var getOwnPropertySymbols = Object.getOwnPropertySymbols;
  var hasOwnProperty = Object.prototype.hasOwnProperty;
  var propIsEnumerable = Object.prototype.propertyIsEnumerable;

  function toObject(val) {
  	if (val === null || val === undefined) {
  		throw new TypeError('Object.assign cannot be called with null or undefined');
  	}

  	return Object(val);
  }

  function shouldUseNative() {
  	try {
  		if (!Object.assign) {
  			return false;
  		}

  		// Detect buggy property enumeration order in older V8 versions.

  		// https://bugs.chromium.org/p/v8/issues/detail?id=4118
  		var test1 = new String('abc');  // eslint-disable-line no-new-wrappers
  		test1[5] = 'de';
  		if (Object.getOwnPropertyNames(test1)[0] === '5') {
  			return false;
  		}

  		// https://bugs.chromium.org/p/v8/issues/detail?id=3056
  		var test2 = {};
  		for (var i = 0; i < 10; i++) {
  			test2['_' + String.fromCharCode(i)] = i;
  		}
  		var order2 = Object.getOwnPropertyNames(test2).map(function (n) {
  			return test2[n];
  		});
  		if (order2.join('') !== '0123456789') {
  			return false;
  		}

  		// https://bugs.chromium.org/p/v8/issues/detail?id=3056
  		var test3 = {};
  		'abcdefghijklmnopqrst'.split('').forEach(function (letter) {
  			test3[letter] = letter;
  		});
  		if (Object.keys(Object.assign({}, test3)).join('') !==
  				'abcdefghijklmnopqrst') {
  			return false;
  		}

  		return true;
  	} catch (err) {
  		// We don't expect any of the above to throw, but better to be safe.
  		return false;
  	}
  }

  var objectAssign = shouldUseNative() ? Object.assign : function (target, source) {
  	var from;
  	var to = toObject(target);
  	var symbols;

  	for (var s = 1; s < arguments.length; s++) {
  		from = Object(arguments[s]);

  		for (var key in from) {
  			if (hasOwnProperty.call(from, key)) {
  				to[key] = from[key];
  			}
  		}

  		if (getOwnPropertySymbols) {
  			symbols = getOwnPropertySymbols(from);
  			for (var i = 0; i < symbols.length; i++) {
  				if (propIsEnumerable.call(from, symbols[i])) {
  					to[symbols[i]] = from[symbols[i]];
  				}
  			}
  		}
  	}

  	return to;
  };

  var defaultGLParameters = {
    alpha: true,
    depth: true,
  };

  let _idTable = 0;

  class GLTool {
    constructor() {
      this._id = `WebGLContext${_idTable++}`;
      this._viewport = [0, 0, 0, 0];
      this._enabledVertexAttribute = [];
      this.identityMatrix = create$1();
      this._normalMatrix = create();
      this._inverseModelViewMatrix = create();
      this._modelMatrix = create$1();
      this._matrix = create$1();
      this._matrixStacks = [];
      this._lastMesh = null;
      this._useWebGL2 = false;
      this._hasArrayInstance = false;
      this._extArrayInstance = false;

      // stats
      this.shaderCount = 0;
      this.meshCount = 0;
      this.bufferCount = 0;
      this.textureCount = 0;
      this.frameBufferCount = 0;

      this.isMobile = isMobile;
    }

    // initialize GL from canvas
    init(mSource, mParameters = {}) {
      if (mSource === null || mSource === undefined) {
        console.error("Canvas not exist");
        return;
      }

      if (mSource instanceof WebGLRenderingContext) {
        this.initWithGL(mSource);
      } else if (mSource instanceof WebGL2RenderingContext) {
        this.webgl2 = true;
        this.initWithGL(mSource);
      }

      if (this.canvas !== undefined && this.canvas !== null) {
        this.destroy();
      }

      this.canvas = mSource;
      this.webgl2 = !!mParameters.webgl2 && checkWebGL2();
      const params = objectAssign({}, defaultGLParameters, mParameters);
      const ctx = getWebGLContext(this.canvas, params, this.webgl2);

      this.initWithGL(ctx);
      this.setSize(window.innerWidth, window.innerHeight);
    }

    // initialize GL from WebGLContext
    initWithGL(gl) {
      if (!this.canvas) {
        this.canvas = ctx.canvas;
      }
      this.gl = gl;

      // extensions
      this.extensions = {};
      for (let i = 0; i < ExtensionsList.length; i++) {
        this.extensions[ExtensionsList[i]] = this.gl.getExtension(
          ExtensionsList[i]
        );
      }

      exposeAttributes(this);
      getAndApplyExtension(gl, "OES_vertex_array_object");
      getAndApplyExtension(gl, "ANGLE_instanced_arrays");
      getAndApplyExtension(gl, "WEBGL_draw_buffers");
      if (this.webgl2) {
        gl.getExtension("EXT_color_buffer_float");
      }

      this.enable(this.DEPTH_TEST);
      this.enable(this.CULL_FACE);
      this.enable(this.BLEND);
      this.enableAlphaBlending();
    }

    // enable gl feature
    enable(mParameter) {
      this.gl.enable(mParameter);
    }

    // disable gl feature
    disable(mParameter) {
      this.gl.disable(mParameter);
    }

    // BLEND MODES

    enableAlphaBlending() {
      const { gl } = this;
      gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    }

    enableAdditiveBlending() {
      const { gl } = this;
      gl.blendFunc(gl.ONE, gl.ONE);
    }

    // set GL size
    setSize(mWidth, mHeight) {
      this._width = mWidth;
      this._height = mHeight;
      this.canvas.width = this._width;
      this.canvas.height = this._height;
      this._aspectRatio = this._width / this._height;

      if (this.gl) {
        this.viewport(0, 0, this._width, this._height);
      }
    }

    // clear the GL context
    clear(r, g, b, a) {
      const { gl } = this;
      gl.clearColor(r, g, b, a);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    }

    // set GL Viewport
    viewport(x, y, w, h) {
      if (checkViewport(this._viewport, w, y, w, h)) {
        this.gl.viewport(x, y, w, h);
        this._viewport = [x, y, w, h];
      }
    }

    // show all extensions
    showExtensions() {
      console.log("Extensions : ", this.extensions);
      for (const ext in this.extensions) {
        if (this.extensions[ext]) {
          console.log(ext, ":", this.extensions[ext]);
        }
      }
    }

    // check if extensions is available
    checkExtension(mExtension) {
      return !!this.extensions[mExtension];
    }

    // get extension by name
    getExtension(mExtension) {
      return this.extensions[mExtension];
    }

    // getter & setters

    get id() {
      return this._id;
    }

    // DESTROY
    destroy() {
      if (this.canvas.parentNode) {
        try {
          this.canvas.parentNode.removeChild(this.canvas);
        } catch (e) {
          console.log("Error : ", e);
        }
      }

      this.canvas = null;
    }
  }

  const GL = new GLTool();

  const addLineNumbers = (string) => {
    const lines = string.split("\n");
    for (let i = 0; i < lines.length; i++) {
      lines[i] = `${i + 1}: ${lines[i]}`;
    }
    return lines.join("\n");
  };

  var vsDefault = "\n#define SHADER_NAME BASIC_VERTEX\nprecision highp float;attribute vec3 aVertexPosition;attribute vec2 aTextureCoord;attribute vec3 aNormal;uniform mat4 uModelMatrix;uniform mat4 uViewMatrix;uniform mat4 uProjectionMatrix;varying vec2 vTextureCoord;varying vec3 vNormal;void main(void){gl_Position=uProjectionMatrix*uViewMatrix*uModelMatrix*vec4(aVertexPosition,1.0);vTextureCoord=aTextureCoord;vNormal=aNormal;}";

  var fsDefault = "precision highp float;varying vec2 vTextureCoord;void main(void){gl_FragColor=vec4(vTextureCoord,0.0,1.0);}";

  class GLShader {
    constructor(mVertexShader, mFragmentShader) {
      this.vertexShader = mVertexShader || vsDefault;
      this.fragmentShader = mFragmentShader || fsDefault;
    }

    // bind Shader to GL
    bind(mGL) {
      this.GL = mGL || GL;
      console.log("Bind Shader :", this.GL.id);

      if (!this.shaderProgram) {
        const vsShader = this._createShaderProgram(this.vertexShader, true);
        const fsShader = this._createShaderProgram(this.fragmentShader, false);
        this._attachShaderProgram(vsShader, fsShader);
      }
    }

    // create shader program
    _createShaderProgram(mShaderStr, isVertexShader) {
      const { GL } = this;
      const { gl } = this.GL;
      const shaderType = isVertexShader ? GL.VERTEX_SHADER : GL.FRAGMENT_SHADER;
      const shader = gl.createShader(shaderType);

      gl.shaderSource(shader, mShaderStr);
      gl.compileShader(shader);

      if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
        console.warn("Error in Shader : ", gl.getShaderInfoLog(shader));
        console.log(addLineNumbers(mShaderStr));
        return null;
      }

      return shader;
    }

    // attach shader program
    _attachShaderProgram(mVertexShader, mFragmentShader) {
      const { gl } = this.GL;

      this.shaderProgram = gl.createProgram();
      gl.attachShader(this.shaderProgram, mVertexShader);
      gl.attachShader(this.shaderProgram, mFragmentShader);
      gl.deleteShader(mVertexShader);
      gl.deleteShader(mFragmentShader);

      gl.linkProgram(this.shaderProgram);
      this.GL.shaderCount++;
    }

    // uniform
    uniform() {
      // console.log("Uniform", arguments.length);
    }

    // destroy shader
    destroy() {
      const { gl } = this.GL;
      gl.deleteProgram(this.shaderProgram);
      this.GL.shaderCount--;
    }
  }

  exports.GL = GL;
  exports.GLShader = GLShader;
  exports.GLTool = GLTool;
  exports.checkWebGL2 = checkWebGL2;
  exports.getWebGLContext = getWebGLContext;

  Object.defineProperty(exports, '__esModule', { value: true });

})));
