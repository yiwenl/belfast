import { GL } from "../core/GL";
import WebglConst from "../utils/WebglConst";
import { formBuffer, getBuffer, getAttribLoc } from "../utils/BufferUtils";

class Mesh {
  constructor(mDrawType = WebglConst.TRIANGLES) {
    this._drawType = mDrawType;

    this._attributes = [];
    this._numInstance = -1;
    this._enabledVertexAttribute = [];

    this._indices = [];
    this._faces = [];
    this._bufferChanged = [];
    this._hasIndexBufferChanged = true;
    this._hasVAO = false;
    this._isInstanced = false;
  }

  bufferVertex(mData, mUsage = WebglConst.STATIC_DRAW) {
    this.bufferData(mData, "aVertexPosition", 3, mUsage);
    return this;
  }

  bufferTexCoord(mArrayTexCoords, mUsage = WebglConstSTATIC_DRAW) {
    this.bufferData(mArrayTexCoords, "aTextureCoord", 2, mUsage);
    return this;
  }

  bufferNormal(mNormals, mUsage = WebglConst.STATIC_DRAW) {
    this.bufferData(mNormals, "aNormal", 3, mUsage);
    return this;
  }

  bufferFlattenData(
    mData,
    mName,
    mItemSize,
    mUsage = WebglConst.STATIC_DRAW,
    isInstanced = false
  ) {
    const data = formBuffer(mData, mItemSize);
    this.bufferData(data, mName, mItemSize, mUsage, isInstanced);
    return this;
  }

  bufferData(
    mData,
    mName,
    mItemSize,
    mUsage = WebglConst.STATIC_DRAW,
    isInstanced = false
  ) {
    let i = 0;
    const usage = mUsage;

    const bufferData = [];
    if (!mItemSize) {
      mItemSize = mData[0].length;
    }
    this._isInstanced = isInstanced || this._isInstanced;

    //	flatten buffer data
    for (i = 0; i < mData.length; i++) {
      for (let j = 0; j < mData[i].length; j++) {
        bufferData.push(mData[i][j]);
      }
    }
    const dataArray = new Float32Array(bufferData);
    const attribute = this.getAttribute(mName);

    if (attribute) {
      //	attribute existed, replace with new data
      attribute.itemSize = mItemSize;
      attribute.dataArray = dataArray;
      attribute.source = mData;
    } else {
      //	attribute not exist yet, create new attribute object
      this._attributes.push({
        name: mName,
        source: mData,
        itemSize: mItemSize,
        usage,
        dataArray,
        isInstanced,
      });
    }

    this._bufferChanged.push(mName);
    return this;
  }

  bufferIndex(mArrayIndices, isDynamic = false) {
    this._usage = isDynamic ? WebglConst.DYNAMIC_DRAW : WebglConst.STATIC_DRAW;
    this._indices = new Uint16Array(mArrayIndices);
    this._numItems = this._indices.length;
    return this;
  }

  bind(mGL) {
    this.GL = mGL || GL;
    this._useVAO = !!this.GL.gl.createVertexArray;
    const mShaderProgram = this.GL.shaderProgram;

    const { gl } = this.GL;
    this.generateBuffers(mShaderProgram);

    if (this.hasVAO) {
      gl.bindVertexArray(this.vao);
    } else {
      this.attributes.forEach((attribute) => {
        gl.bindBuffer(gl.ARRAY_BUFFER, attribute.buffer);
        const attrPosition = attribute.attrPosition;
        gl.vertexAttribPointer(
          attrPosition,
          attribute.itemSize,
          gl.FLOAT,
          false,
          0,
          0
        );

        if (attribute.isInstanced) {
          gl.vertexAttribDivisor(attrPosition, 1);
        }
      });

      //	BIND INDEX BUFFER
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.iBuffer);
    }
  }

  // create buffers
  generateBuffers(mShaderProgram) {
    const { gl } = this.GL;
    if (this._bufferChanged.length == 0) {
      return;
    }

    if (this._useVAO) {
      //	IF SUPPORTED, CREATE VAO

      //	CREATE & BIND VAO
      if (!this._vao) {
        this._vao = gl.createVertexArray();
      }

      gl.bindVertexArray(this._vao);

      //	UPDATE BUFFERS
      this._attributes.forEach((attrObj) => {
        if (this._bufferChanged.indexOf(attrObj.name) !== -1) {
          const buffer = getBuffer(attrObj, gl);
          gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
          gl.bufferData(gl.ARRAY_BUFFER, attrObj.dataArray, attrObj.usage);

          const attrPosition = getAttribLoc(gl, mShaderProgram, attrObj.name);
          gl.enableVertexAttribArray(attrPosition);
          gl.vertexAttribPointer(
            attrPosition,
            attrObj.itemSize,
            gl.FLOAT,
            false,
            0,
            0
          );
          attrObj.attrPosition = attrPosition;

          if (attrObj.isInstanced) {
            gl.vertexAttribDivisor(attrPosition, 1);
          }
        }
      });

      //	check index buffer
      this._updateIndexBuffer();

      //	UNBIND VAO
      gl.bindVertexArray(null);

      this._hasVAO = true;
    } else {
      //	ELSE, USE TRADITIONAL METHOD
      this._attributes.forEach((attrObj) => {
        //	SKIP IF BUFFER HASN'T CHANGED
        if (this._bufferChanged.indexOf(attrObj.name) !== -1) {
          const buffer = getBuffer(attrObj, gl);
          gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
          gl.bufferData(gl.ARRAY_BUFFER, attrObj.dataArray, attrObj.usage);

          const attrPosition = getAttribLoc(gl, mShaderProgram, attrObj.name);
          gl.enableVertexAttribArray(attrPosition);
          gl.vertexAttribPointer(
            attrPosition,
            attrObj.itemSize,
            gl.FLOAT,
            false,
            0,
            0
          );
          attrObj.attrPosition = attrPosition;

          if (attrObj.isInstanced) {
            gl.vertexAttribDivisor(attrPosition, 1);
          }
        }
      });

      this._updateIndexBuffer();
    }

    this._hasIndexBufferChanged = false;
    this._bufferChanged = [];
  }

  _updateIndexBuffer() {
    const { gl } = this.GL;
    if (this._hasIndexBufferChanged) {
      if (!this.iBuffer) {
        this.iBuffer = gl.createBuffer();
      }
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.iBuffer);
      gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, this._indices, this._usage);
      this.iBuffer.itemSize = 1;
      this.iBuffer.numItems = this._numItems;
    }
  }

  // unbind buffers
  unbind() {
    const { gl } = this.GL;
    if (this._useVAO) {
      gl.bindVertexArray(null);
    }

    this._attributes.forEach((attribute) => {
      if (attribute.isInstanced) {
        gl.vertexAttribDivisor(attribute.attrPosition, 0);
      }
    });
  }

  // generate face for touch detection
  generateFaces() {
    let ia, ib, ic;
    let a, b, c;
    const { vertices } = this;

    for (let i = 0; i < this._indices.length; i += 3) {
      ia = this._indices[i];
      ib = this._indices[i + 1];
      ic = this._indices[i + 2];

      a = vertices[ia];
      b = vertices[ib];
      c = vertices[ic];

      const face = {
        indices: [ia, ib, ic],
        vertices: [a, b, c],
      };

      this._faces.push(face);
    }
  }

  // get attribute by name
  getAttribute(mName) {
    return this._attributes.find((a) => a.name === mName);
  }

  // get attribute source by name
  getSource(mName) {
    const attr = this.getAttribute(mName);
    return attr ? attr.source : [];
  }

  // destroy
  destroy() {
    const { gl } = this.GL;
    this.attributes.forEach((attr) => {
      gl.deleteBuffer(attr.buffer);
      attr.source = [];
      attr.dataArray = [];
    });
    if (this.iBuffer) {
      gl.deleteBuffer(this.iBuffer);
    }
    gl.deleteVertexArray(this._vao);

    // resetting
    this._attributes = [];
    this._enabledVertexAttribute = [];
    this._indices = [];
    this._faces = [];
    this._bufferChanged = [];
  }

  // getter & setters

  get drawType() {
    return this._drawType;
  }

  get vertices() {
    return this.getSource("aVertexPosition");
  }

  get normals() {
    return this.getSource("aNormal");
  }

  get coords() {
    return this.getSource("aTextureCoord");
  }

  get indices() {
    return this._indices;
  }

  get vertexSize() {
    return this.vertices.length;
  }

  get faces() {
    return this._faces;
  }

  get attributes() {
    return this._attributes;
  }

  get hasVAO() {
    return this._hasVAO;
  }

  get vao() {
    return this._vao;
  }

  get numInstance() {
    return this._numInstance;
  }

  get isInstanced() {
    return this._isInstanced;
  }
}

export { Mesh };
