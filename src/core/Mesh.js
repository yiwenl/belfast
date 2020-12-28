import { WebGLConst } from "../utils/WebGLConst";
import { flatten, getBuffer, getAttribLoc } from "../utils/BufferUtils";

function Mesh(mDrawType = WebGLConst.TRIANGLES) {
  this.drawType = mDrawType;

  // PUBLIC PROPERTIES
  this.isInstanced = false;
  this.numItems = 0;

  // PRIVATE PROPERTIES
  let _attributes = [];
  let _bufferChanged = [];
  let _hasIndexBufferChanged = true;

  let _vao;
  let _usage;
  let _indices;
  let _indexBuffer;
  let _GL;

  /**
   * add or update an attribute
   *
   * @param {array} mData the data of the attribute, array of array
   * @param {string} mName the name of the attribute
   * @param {number} mItemSize the size of each element
   * @param {GLenum} mUsage the usage of the attribute, static or dynamic
   * @param {GLenum} isInstanced if the attribute is an instanced attrbute
   */
  this.bufferData = function(
    mData,
    mName,
    mItemSize,
    mUsage = WebGLConst.STATIC_DRAW,
    isInstanced = false
  ) {
    let bufferData;
    let orgData = [];
    if (typeof mData[0] === "number") {
      bufferData = mData;
      if (mItemSize === undefined) {
        console.error("Missing element size for flatten data :", mName);
        return this;
      }

      for (let i = 0; i < bufferData.length; i += mItemSize) {
        const a = [];
        for (let j = 0; j < mItemSize; j++) {
          a.push(bufferData[i + j]);
        }
        orgData.push(a);
      }
    } else {
      orgData = mData;
      bufferData = flatten(mData);
    }

    const itemSize = mItemSize === undefined ? mData[0].length : mItemSize;
    return bufferFlattenData(
      bufferData,
      mData,
      mName,
      itemSize,
      mUsage,
      isInstanced
    );
  };

  /**
   * Add or Update the vertex position attribute
   *
   * @param {array} mData the data of the vertex positions
   * @param {GLenum} mUsage the usage of the attribute, static or dynamic
   */
  this.bufferVertex = function(mData, mUsage = WebGLConst.STATIC_DRAW) {
    return this.bufferData(mData, "aVertexPosition", 3, mUsage);
  };

  /**
   * Add or Update the texture coordinate attribute
   *
   * @param {array} mData the data of the texture coordinate
   * @param {GLenum} mUsage the usage of the attribute, static or dynamic
   */
  this.bufferTexCoord = function(mData, mUsage = WebGLConst.STATIC_DRAW) {
    return this.bufferData(mData, "aTextureCoord", 2, mUsage);
  };

  /**
   * Add or Update the vertex normal attribute
   *
   * @param {array} mData the data of the normal
   * @param {GLenum} mUsage the usage of the attribute, static or dynamic
   */
  this.bufferNormal = function(mData, mUsage = WebGLConst.STATIC_DRAW) {
    return this.bufferData(mData, "aNormal", 3, mUsage);
  };

  /**
   * Add or Update the index buffer
   *
   * @param {array} mData the data of the index buffer
   * @param {GLenum} mUsage the usage of the attribute, static or dynamic
   */
  this.bufferIndex = function(mData, mUsage = WebGLConst.STATIC_DRAW) {
    _usage = mUsage;
    _indices = new Uint16Array(mData);
    this.numItems = _indices.length;
    _hasIndexBufferChanged = true;
    return this;
  };

  /**
   * Bind the buffers of current Mesh
   *
   * @param {GL} mGL the GLTool instance
   */
  this.bind = function(mGL) {
    if (mGL !== undefined && _GL !== undefined && mGL !== _GL) {
      console.error(
        "this mesh has been bind to a different WebGL Rendering Context"
      );
      return;
    }

    _GL = mGL || GL;
    const { gl } = _GL;
    generateBuffers();
    gl.bindVertexArray(_vao);

    this.vertexSize = this.getSource("aVertexPosition").length;
  };

  this.unbind = function() {};

  /**
   * Find an attribute by name
   *
   * @param {string} mName the name of the attribute
   * @returns {object} the attribute object
   */
  this.getAttribute = function(mName) {
    return _attributes.find((a) => a.name === mName);
  };

  /**
   * get all attribtues
   *
   * @returns {array} the array of attributes
   */
  this.getAttributes = function() {
    return _attributes;
  };

  /**
   * Find data source by name
   *
   * @param {string} mName the name of the attribute
   * @returns {[array]} the source data of the attribute ( array of arrays )
   */
  this.getSource = function(mName) {
    const attr = this.getAttribute(mName);
    return attr ? attr.source : [];
  };

  /**
   * Destroy all buffers
   *
   */
  this.destroy = function() {
    const { gl } = _GL;
    _attributes.forEach((attr) => {
      gl.deleteBuffer(attr.buffer);
      attr.source = [];
      attr.dataArray = [];
      _GL.bufferCount--;
    });
    if (_indexBuffer) {
      gl.deleteBuffer(_indexBuffer);
      _GL.bufferCount--;
    }
    gl.deleteVertexArray(_vao);

    // resetting
    _attributes = [];
    _indices = [];
    _bufferChanged = [];
    // _enabledVertexAttribute = [];
  };

  /**
   * add or update an attribute
   *
   * @param {array} mData the data of the attribute
   * @param {string} mName the name of the attribute
   * @param {number} mItemSize the size of each element
   * @param {GLenum} mUsage the usage of the attribute, static or dynamic
   * @param {GLenum} isInstanced if the attribute is an instanced attrbute
   */
  const bufferFlattenData = (
    mData,
    mDataOrg,
    mName,
    mItemSize,
    mUsage = WebGLConst.STATIC_DRAW,
    isInstanced = false
  ) => {
    const usage = mUsage;
    this.isInstanced = isInstanced || this.isInstanced;

    const dataArray = new Float32Array(mData);
    const attribute = this.getAttribute(mName);

    if (attribute) {
      //	attribute existed, replace with new data
      attribute.itemSize = mItemSize;
      attribute.dataArray = dataArray;
      attribute.source = mDataOrg;
    } else {
      //	attribute not exist yet, create new attribute object
      _attributes.push({
        name: mName,
        source: mDataOrg,
        itemSize: mItemSize,
        usage,
        dataArray,
        isInstanced,
      });
    }

    _bufferChanged.push(mName);
    return this;
  };

  /**
   * Generate new buffers
   *
   */
  const generateBuffers = () => {
    const { shaderProgram, gl } = _GL;
    if (_bufferChanged.length == 0) {
      return;
    }

    if (!_vao) {
      _vao = gl.createVertexArray();
    }

    gl.bindVertexArray(_vao);

    //	UPDATE BUFFERS
    _attributes.forEach((attrObj) => {
      if (_bufferChanged.indexOf(attrObj.name) !== -1) {
        const buffer = getBuffer(attrObj, _GL);
        gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
        gl.bufferData(gl.ARRAY_BUFFER, attrObj.dataArray, attrObj.usage);

        const attrPosition = getAttribLoc(gl, shaderProgram, attrObj.name);
        if (attrPosition >= 0) {
          gl.enableVertexAttribArray(attrPosition);
          gl.vertexAttribPointer(
            attrPosition,
            attrObj.itemSize,
            gl.FLOAT,
            false,
            0,
            0
          );
        }
        attrObj.attrPosition = attrPosition;

        if (attrObj.isInstanced) {
          gl.vertexAttribDivisor(attrPosition, 1);
        }
      }
    });

    //	check index buffer
    _updateIndexBuffer();

    //	UNBIND VAO
    gl.bindVertexArray(null);

    _hasIndexBufferChanged = false;
    _bufferChanged = [];
  };

  /**
   * Update Index Buffer
   *
   */
  const _updateIndexBuffer = () => {
    const { gl } = _GL;
    if (_hasIndexBufferChanged) {
      if (!_indexBuffer) {
        _indexBuffer = gl.createBuffer();
        _GL.bufferCount++;
      }
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, _indexBuffer);
      gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, _indices, _usage);
    }
  };
}

export { Mesh };
