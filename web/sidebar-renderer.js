// Native-resolution rendering of egui geometry, not a copy of another canvas.
export function backingSize(width,height,dpr) {
  return [Math.max(1,Math.round(width*dpr)),Math.max(1,Math.round(height*dpr))];
}
export function clipPixels(clip,scale,width,height) {
  const left=Math.max(0,Math.floor(clip[0]*scale)),top=Math.max(0,Math.floor(clip[1]*scale));
  const right=Math.min(width,Math.ceil(clip[2]*scale)),bottom=Math.min(height,Math.ceil(clip[3]*scale));
  return [left,height-bottom,Math.max(0,right-left),Math.max(0,bottom-top)];
}
export function createSidebarRenderer(canvas) {
  const gl=canvas.getContext('webgl2',{alpha:false,antialias:false,premultipliedAlpha:true});
  if(!gl)throw Error('This browser cannot create the study panel renderer.');
  const shader=(type,source)=>{
    const value=gl.createShader(type);gl.shaderSource(value,source);gl.compileShader(value);
    if(!gl.getShaderParameter(value,gl.COMPILE_STATUS))throw Error(gl.getShaderInfoLog(value));
    return value;
  };
  const vertex=shader(gl.VERTEX_SHADER,`#version 300 es
in vec2 a_position;in vec2 a_uv;in vec4 a_color;
uniform vec2 u_size;out vec2 v_uv;out vec4 v_color;
void main(){v_uv=a_uv;v_color=a_color;gl_Position=vec4(2.0*a_position.x/u_size.x-1.0,1.0-2.0*a_position.y/u_size.y,0,1);}`);
  const fragment=shader(gl.FRAGMENT_SHADER,`#version 300 es
precision mediump float;in vec2 v_uv;in vec4 v_color;uniform sampler2D u_texture;out vec4 out_color;
void main(){out_color=v_color*texture(u_texture,v_uv);}`);
  const program=gl.createProgram();gl.attachShader(program,vertex);gl.attachShader(program,fragment);gl.linkProgram(program);
  if(!gl.getProgramParameter(program,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(program));
  gl.deleteShader(vertex);gl.deleteShader(fragment);
  const vao=gl.createVertexArray(),vertices=gl.createBuffer(),indices=gl.createBuffer(),textures=new Map();
  gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,vertices);gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,indices);
  for(const [name,count,offset] of [['a_position',2,0],['a_uv',2,8],['a_color',4,16]]) {
    const location=gl.getAttribLocation(program,name);gl.enableVertexAttribArray(location);gl.vertexAttribPointer(location,count,gl.FLOAT,false,32,offset);
  }
  const size=gl.getUniformLocation(program,'u_size');
  return {
    textureLimit:gl.getParameter(gl.MAX_TEXTURE_SIZE),
    resize(width,height,dpr){
      const [w,h]=backingSize(width,height,dpr);
      if(canvas.width!==w||canvas.height!==h){canvas.width=w;canvas.height=h;}
    },
    paint(frame){
      gl.useProgram(program);gl.bindVertexArray(vao);gl.activeTexture(gl.TEXTURE0);
      gl.pixelStorei(gl.UNPACK_ALIGNMENT,1);
      for(const image of frame.textures){
        let texture=textures.get(image.id);
        if(!texture){texture=gl.createTexture();textures.set(image.id,texture);}
        gl.bindTexture(gl.TEXTURE_2D,texture);
        gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,image.nearestMin?gl.NEAREST:gl.LINEAR);
        gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,image.nearest?gl.NEAREST:gl.LINEAR);
        const wrap=image.wrap==='Repeat'?gl.REPEAT:image.wrap==='MirroredRepeat'?gl.MIRRORED_REPEAT:gl.CLAMP_TO_EDGE;
        gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,wrap);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,wrap);
        if(image.x===undefined)gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,image.width,image.height,0,gl.RGBA,gl.UNSIGNED_BYTE,image.pixels);
        else gl.texSubImage2D(gl.TEXTURE_2D,0,image.x,image.y,image.width,image.height,gl.RGBA,gl.UNSIGNED_BYTE,image.pixels);
      }
      gl.viewport(0,0,canvas.width,canvas.height);gl.disable(gl.SCISSOR_TEST);gl.clearColor(17/255,20/255,24/255,1);gl.clear(gl.COLOR_BUFFER_BIT);
      gl.enable(gl.BLEND);gl.blendEquation(gl.FUNC_ADD);gl.blendFunc(gl.ONE,gl.ONE_MINUS_SRC_ALPHA);
      gl.disable(gl.DEPTH_TEST);gl.disable(gl.CULL_FACE);gl.enable(gl.SCISSOR_TEST);
      gl.uniform2f(size,frame.width,frame.height);
      for(const mesh of frame.meshes){
        const texture=textures.get(mesh.texture);if(!texture)continue;
        const clip=clipPixels(mesh.clip,frame.pixelsPerPoint,canvas.width,canvas.height);
        if(!clip[2]||!clip[3])continue;
        gl.scissor(...clip);gl.bindTexture(gl.TEXTURE_2D,texture);
        gl.bindBuffer(gl.ARRAY_BUFFER,vertices);gl.bufferData(gl.ARRAY_BUFFER,mesh.vertices,gl.STREAM_DRAW);
        gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,indices);gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,mesh.indices,gl.STREAM_DRAW);
        gl.drawElements(gl.TRIANGLES,mesh.indices.length,gl.UNSIGNED_INT,0);
      }
      for(const id of frame.free){const texture=textures.get(id);if(texture)gl.deleteTexture(texture);textures.delete(id);}
    },
    destroy(){for(const texture of textures.values())gl.deleteTexture(texture);textures.clear();gl.deleteBuffer(vertices);gl.deleteBuffer(indices);gl.deleteVertexArray(vao);gl.deleteProgram(program);},
  };
}
