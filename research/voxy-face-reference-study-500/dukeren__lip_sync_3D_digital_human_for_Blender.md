# Facial rendering source scan: dukeren/lip_sync_3D_digital_human_for_Blender

Repository: https://github.com/dukeren/lip_sync_3D_digital_human_for_Blender

Pinned source revision: 98ee84664dee43b94428a4520f3305e447fb31dc

Discovery topic: lip sync 3D

Description: lip_sync_3D_digital_human

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"lip": ["bg/Lip Sync Flow.svg", "lip_sync.py", "lip_sync_core.py", "lip_sync_idle_animation_generator.py", "lipsync_animation_handler.py", "lipsync_cleaner.py"]}

## Source file: lip_sync.py

{
  "path": "lip_sync.py",
  "blob_sha": "14dc8f533334c0da1ed6cc2cf47030c1464e8f00",
  "sha256": "bc5b3405d55101e4345ce3a53904dcd66a7d1a6084869aa835c063fa4c58d564",
  "lines": 329,
  "bytes": 13617,
  "selection_score": 15,
  "keyword_line_counts": {
    "mouth": 10,
    "lip": 84
  },
  "url": "https://github.com/dukeren/lip_sync_3D_digital_human_for_Blender/blob/98ee84664dee43b94428a4520f3305e447fb31dc/lip_sync.py",
  "function_signature_candidates": [
    {
      "line": 61,
      "text": "def execute(self, context):"
    },
    {
      "line": 72,
      "text": "def execute(self, context):"
    },
    {
      "line": 81,
      "text": "def execute(self, context):"
    },
    {
      "line": 91,
      "text": "def analyze_and_apply(self, context, audio_file):"
    },
    {
      "line": 169,
      "text": "def modal(self, context, event):"
    },
    {
      "line": 191,
      "text": "def execute(self, context):"
    },
    {
      "line": 202,
      "text": "def cancel(self, context):"
    },
    {
      "line": 210,
      "text": "def invoke(self, context, event):"
    },
    {
      "line": 221,
      "text": "def execute(self, context):"
    },
    {
      "line": 233,
      "text": "def execute(self, context):"
    },
    {
      "line": 242,
      "text": "def execute(self, context):"
    },
    {
      "line": 251,
      "text": "def execute(self, context):"
    },
    {
      "line": 259,
      "text": "def draw_panel(context, layout):"
    },
    {
      "line": 318,
      "text": "def register():"
    },
    {
      "line": 323,
      "text": "def unregister():"
    }
  ]
}

```text

import sys
import bpy
import os
import logging
import time
from bpy.props import StringProperty, BoolProperty, FloatProperty, IntProperty, EnumProperty, CollectionProperty, PointerProperty
from bpy_extras.io_utils import ImportHelper

sys.path.append(os.path.dirname(__file__))
from lip_sync_core import LipSyncCore
from lip_sync_idle_animation_generator import IdleAnimationGenerator

# Configure logging
logger = logging.getLogger("LipSyncLogger")
logger.setLevel(logging.DEBUG)

handler = logging.StreamHandler()
handler.setLevel(logging.DEBUG)

formatter = logging.Formatter('%(asctime)s - %(levelname)s - %(message)s')
handler.setFormatter(formatter)

logger.addHandler(handler)

class IdleAnimation(bpy.types.PropertyGroup):
    name: StringProperty(name="名称")
    object: PointerProperty(name="对象", type=bpy.types.Object)
    shape_key: StringProperty(name="形态键")
    min_interval: FloatProperty(name="最小间隔", default=1.0, min=0.1)
    max_interval: FloatProperty(name="最大间隔", default=5.0, min=0.1)
    min_duration: FloatProperty(name="最小持续时间", default=0.1, min=0.1)
    max_duration: FloatProperty(name="最大持续时间", default=1.0, min=0.1)

class LipSyncProperties(bpy.types.PropertyGroup):
    audio_file: StringProperty(name="音频文件", default="")
    is_listening: BoolProperty(name="监听音频", default=False)
    frame_rate: FloatProperty(name="帧率", default=24.0)
    silence_threshold: FloatProperty(name="静音阈值", default=0.01, min=0.0, max=1.0)
    max_silence_frames: IntProperty(name="最大静音帧数", default=5, min=1)
    monitor_folder: StringProperty(name="监听文件夹", default=os.path.join(os.path.dirname(__file__), 'Voice'), subtype='DIR_PATH')
    is_monitoring: BoolProperty(name="正在监听", default=False)
    idle_animations: CollectionProperty(type=IdleAnimation)
    active_idle_animation: IntProperty(default=0)
    custom_frames: IntProperty(name="自定义帧数", description="设置自定义的总帧数，留空或设为0则使用场景的结束帧", min=0, default=0)
    mouth_object: PointerProperty(name="唇型对象", type=bpy.types.Object)
    language: EnumProperty(
        name="语言",
        items=[
            ('chinese', "中文", "使用中文音素到口型映射"),
            ('english', "英文", "使用英文音素到口型映射")
        ],
        default='chinese'
    )

class LIPSYNC_OT_select_audio(bpy.types.Operator, ImportHelper):
    bl_idname = "lipsync.select_audio"
    bl_label = "选择音频文件"
    
    filter_glob: StringProperty(default="*.wav;*.mp3", options={'HIDDEN'})

    def execute(self, context):
        context.scene.lip_sync.audio_file = self.filepath
        logger.info(f"选择的音频文件: {self.filepath}")
        return {'FINISHED'}

class LIPSYNC_OT_select_monitor_folder(bpy.types.Operator, ImportHelper):
    bl_idname = "lipsync.select_monitor_folder"
    bl_label = "选择监听文件夹"
    
    directory: StringProperty(subtype='DIR_PATH')

    def execute(self, context):
        context.scene.lip_sync.monitor_folder = self.directory
        logger.info(f"选择的监听文件夹: {self.directory}")
        return {'FINISHED'}

class LIPSYNC_OT_analyze_audio(bpy.types.Operator):
    bl_idname = "lipsync.analyze_audio"
    bl_label = "分析音频"

    def execute(self, context):
        audio_file = context.scene.lip_sync.audio_file
        if not audio_file:
            self.report({'ERROR'}, "未选择音频文件")
            logger.error("错误: 未选择音频文件")
            return {'CANCELLED'}
        
        self.analyze_and_apply(context, audio_file)
        return {'FINISHED'}

    def analyze_and_apply(self, context, audio_file):
        lip_sync_core = LipSyncCore(
            frame_rate=context.scene.lip_sync.frame_rate,
            silence_threshold=context.scene.lip_sync.silence_threshold,
            max_silence_frames=context.scene.lip_sync.max_silence_frames,
            language=context.scene.lip_sync.language
        )
        
        mouth_object = context.scene.lip_sync.mouth_object
        
        if mouth_object is None or mouth_object.type != 'MESH':
            self.report({'ERROR'}, f"未选择有效的唇形网格对象")
            logger.error(f"错误: 未选择有效的唇型网格对象")
            return
        
        if mouth_object.data.shape_keys is None:
            self.report({'ERROR'}, f"口型对象 '{mouth_object.name}' 没有形态键")
            logger.error(f"错误: 口型对象 '{mouth_object.name}' 没有形态键")
            return
        
        logger.info(f"口型对象: {mouth_object.name}, 类型: {mouth_object.type}")
        
        try:
            # 分析音频并生成口型数据
            visemes = lip_sync_core.analyze_audio(audio_file)
            logger.info(f"生成的visemes数量: {len(visemes)}")
            
            # 应用口型数据到网格对象
            action_name = f"LipSync_{int(time.time())}"
            lip_sync_action = lip_sync_core.apply_visemes_to_mesh(mouth_object, visemes, action_name)
            logger.info(f"创建的动作: {lip_sync_action.name}")
            
            # 创建NLA轨道和条带
            track_name = f"LipSync_Track_{int(time.time())}"
            strip_name = f"LipSync_Strip_{int(time.time())}"
            lip_sync_core.create_nla_track(mouth_object, lip_sync_action, track_name, strip_name)
            logger.info(f"创建了NLA轨道: {track_name}, 条带: {strip_name}")
            
            # 清除之前的音频(如果有)
            if context.scene.sequence_editor:
                for seq in context.scene.sequence_editor.sequences_all:
                    if seq.type == 'SOUND':
                        context.scene.sequence_editor.sequences.remove(seq)
                logger.info("清除了之前的音频")

            # 插入新的音频
            if not context.scene.sequence_editor:
                context.scene.sequence_editor_create()

            sound_strip = context.scene.sequence_editor.sequences.new_sound(
                name="LipSync Audio",
                filepath=audio_file,
                channel=1,
                frame_start=1
            )
            logger.info(f"插入了新的音频: {audio_file}")
            logger.info(f"音频条带名称: {sound_strip.name}, 持续时间: {sound_strip.frame_duration} 帧")

            # 可选：设置音量或其他属性
            sound_strip.volume = 1.0  # 设置音量为100%

            self.report({'INFO'}, f"完成了唇形同步,并插入了音频")
            logger.info(f"完成了唇形同步,并插入了音频")

        except Exception as e:
            self.report({'ERROR'}, f"处理过程中发生错误: {str(e)}")
            logger.error(f"处理过程中发生错误: {str(e)}")
            logger.error(f"错误类型: {type(e).__name__}")
            logger.error(f"错误发生位置: {e.__traceback__.tb_frame.f_code.co_filename}, 行号: {e.__traceback__.tb_lineno}")
            return {'CANCELLED'}

class LIPSYNC_OT_monitor_folder(bpy.types.Operator):
    bl_idname = "lipsync.monitor_folder"
    bl_label = "监听文件夹"
    
    _timer = None
    _last_files = set()

    def modal(self, context, event):
        if event.type == 'TIMER':
            if not context.scene.lip_sync.is_listening:
                self.cancel(context)
                return {'CANCELLED'}
            
            monitor_folder = context.scene.lip_sync.monitor_folder
            current_files = set(os.listdir(monitor_folder))
            new_files = current_files - self._last_files
            
            if new_files:
                for file in new_files:
                    file_path = os.path.join(monitor_folder, file)
                    if file.lower().endswith(('.wav', '.mp3')):
                        logger.info(f"检测到新的音频文件: {file}")
                        context.scene.lip_sync.audio_file = file_path
                        bpy.ops.lipsync.analyze_audio()
                
                self._last_files = current_files
        
        return {'PASS_THROUGH'}

    def execute(self, context):
        wm = context.window_manager
        self._timer = wm.event_timer_add(1.0, window=context.window)
        wm.modal_handler_add(self)
        self._last_files = set(os.listdir(context.scene.lip_sync.monitor_folder))
        logger.info(f"开始监听文件夹: {context.scene.lip_sync.monitor_folder}")
        logger.info(f"初始文件列表: {self._last_files}")
        logger.info(f"监听状态: {context.scene.lip_sync.is_listening}")
        context.scene.lip_sync.is_monitoring = True
        return {'RUNNING_MODAL'}

    def cancel(self, context):
        wm = context.window_manager
        if self._timer is not None:
            wm.event_timer_remove(self._timer)
            self._timer = None
        logger.info("停止监听文件夹")
        context.scene.lip_sync.is_monitoring = False

    def invoke(self, context, event):
        if context.scene.lip_sync.is_monitoring:
            self.cancel(context)
            return {'CANCELLED'}
        else:
            return self.execute(context)

class LIPSYNC_OT_add_idle_animation(bpy.types.Operator):
    bl_idname = "lipsync.add_idle_animation"
    bl_label = "添加闲置动画"

    def execute(self, context):
        idle_anims = context.scene.lip_sync.idle_animations
        new_anim = idle_anims.add()
        new_anim.name = f"IdleAnimation{len(idle_anims)}"
        return {'FINISHED'}

class LIPSYNC_OT_remove_idle_animation(bpy.types.Operator):
    bl_idname = "lipsync.remove_idle_animation"
    bl_label = "移除闲置动画"

    index: IntProperty()

    def execute(self, context):
        idle_anims = context.scene.lip_sync.idle_animations
        idle_anims.remove(self.index)
        return {'FINISHED'}

class LIPSYNC_OT_generate_idle_animations(bpy.types.Operator):
    bl_idname = "lipsync.generate_idle_animations"
    bl_label = "生成闲置动画"

    def execute(self, context):
        generator = IdleAnimationGenerator()
        return generator.generate_idle_animations(context)

class LIPSYNC_OT_clear_idle_animations(bpy.types.Operator):
    bl_idname = "lipsync.clear_idle_animations"
    bl_label = "清除闲置动画"
    bl_description = "清除所有对象的闲置动画"

    def execute(self, context):
        generator = IdleAnimationGenerator()
        for idle_anim in context.scene.lip_sync.idle_animations:
            if idle_anim.object:
                generator.clear_all_idle_animations(idle_anim.object)
        self.report({'INFO'}, "已清除所有闲置动画")
        return {'FINISHED'}

def draw_panel(context, layout):
    lip_sync = context.scene.lip_sync

    row = layout.row()
    row.prop(lip_sync, "is_listening", text="检测音频")
    if lip_sync.is_listening:
        row.operator("lipsync.select_monitor_folder", text="选择检测文件夹")
        layout.prop(lip_sync, "monitor_folder", text="")
        if not lip_sync.monitor_folder:
            layout.label(text="请选择监听文件夹", icon='ERROR')
        else:
            if lip_sync.is_monitoring:
                layout.operator("lipsync.monitor_folder", text="停止检测")
            else:
                layout.operator("lipsync.monitor_folder", text="开始检测")
    else:
        row.operator("lipsync.select_audio", text="选择音频文件")
        layout.prop(lip_sync, "audio_file", text="")
        layout.operator("lipsync.analyze_audio", text="分析并应用")

    layout.separator()
    layout.prop(lip_sync, "mouth_object", text="唇型对象")
    layout.prop(lip_sync, "frame_rate", text="帧率")
    layout.prop(lip_sync, "silence_threshold", text="静音阈值")
    layout.prop(lip_sync, "max_silence_frames", text="最大静音帧数")
    layout.prop(lip_sync, "language", text="语言")

    layout.separator()
    layout.label(text="闲置动画:")
    layout.prop(lip_sync, "custom_frames", text="自定义帧数")
    for i, anim in enumerate(lip_sync.idle_animations):
        box = layout.box()
        row = box.row()
        row.prop(anim, "name", text="")
        row.operator("lipsync.remove_idle_animation", text="", icon='X').index = i
        box.prop(anim, "object", text="对象")
        box.prop(anim, "shape_key", text="形态键")
        box.prop(anim, "min_interval", text="最小间隔")
        box.prop(anim, "max_interval", text="最大间隔")
        box.prop(anim, "min_duration", text="最小持续时间")
        box.prop(anim, "max_duration", text="最大持续时间")

    layout.operator("lipsync.add_idle_animation", text="添加闲置动画")
    layout.operator("lipsync.generate_idle_animations", text="生成闲置动画")
    layout.operator("lipsync.clear_idle_animations", text="清除闲置动画")

classes = (
    IdleAnimation,
    LipSyncProperties,
    LIPSYNC_OT_select_audio,
    LIPSYNC_OT_analyze_audio,
    LIPSYNC_OT_select_monitor_folder,
    LIPSYNC_OT_monitor_folder,
    LIPSYNC_OT_add_idle_animation,
    LIPSYNC_OT_remove_idle_animation,
    LIPSYNC_OT_generate_idle_animations,
    LIPSYNC_OT_clear_idle_animations,
)

def register():
    for cls in classes:
        bpy.utils.register_class(cls)
    bpy.types.Scene.lip_sync = bpy.props.PointerProperty(type=LipSyncProperties)

def unregister():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
    del bpy.types.Scene.lip_sync

if __name__ == "__main__":
    register()

```

## Source file: lip_sync_core.py

{
  "path": "lip_sync_core.py",
  "blob_sha": "36a43d14ef2508d16270a7f9814369e4cd818f9e",
  "sha256": "ef27a463159b85ec6907bf9955458730676bce05d2a45d510221cf8b8dd3022f",
  "lines": 167,
  "bytes": 7540,
  "selection_score": 15,
  "keyword_line_counts": {
    "lip": 16
  },
  "url": "https://github.com/dukeren/lip_sync_3D_digital_human_for_Blender/blob/98ee84664dee43b94428a4520f3305e447fb31dc/lip_sync_core.py",
  "function_signature_candidates": [
    {
      "line": 43,
      "text": "def __init__(self, frame_rate=24.0, silence_threshold=0.01, max_silence_frames=5, language='chinese'):"
    },
    {
      "line": 50,
      "text": "def set_language(self, language):"
    },
    {
      "line": 59,
      "text": "def analyze_audio(self, audio_file):"
    },
    {
      "line": 76,
      "text": "def generate_visemes(self, total_frames, mel_spec, chroma):"
    },
    {
      "line": 106,
      "text": "def apply_visemes_to_mesh(self, obj, visemes, action_name):"
    },
    {
      "line": 134,
      "text": "def create_nla_track(self, obj, action, track_name, strip_name):"
    },
    {
      "line": 153,
      "text": "def process_audio_and_apply_lipsync(self, obj, audio_file):"
    }
  ]
}

```text

import librosa
import numpy as np
import bpy
import logging
import time  # Add this line to import the time module

logger = logging.getLogger("LipSyncLogger")
logger.setLevel(logging.DEBUG)

# 如果还没有处理器，添加一个控制台处理器
if not logger.handlers:
    ch = logging.StreamHandler()
    ch.setLevel(logging.DEBUG)
    formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')
    ch.setFormatter(formatter)
    logger.addHandler(ch)

# 英文音素到口型映射
ENGLISH_PHONEME_TO_VISEME = {
    'a': 'A', 'i': 'I', 'u': 'U', 'e': 'E', 'o': 'O',
    'm': 'U', 'b': 'U', 'p': 'U', 'f': 'U', 'v': 'U',
    's': 'I', 'z': 'I', 'th': 'I', 'ch': 'I', 'sh': 'I',
    'j': 'I', 'zh': 'I', 'l': 'I', 'r': 'I', 'y': 'I',
    'w': 'U', 'ng': 'E', 'h': 'A', 'k': 'E', 'g': 'E',
}

# 中文音素到口型映射
CHINESE_PHONEME_TO_VISEME = {
    'a': 'A', 'i': 'I', 'u': 'U', 'e': 'E', 'o': 'O', 'ü': 'I',
    'b': 'U', 'p': 'U', 'm': 'U', 'f': 'U',
    'd': 'I', 't': 'I', 'n': 'I', 'l': 'I',
    'g': 'E', 'k': 'E', 'h': 'E',
    'j': 'I', 'q': 'I', 'x': 'I',
    'z': 'I', 'c': 'I', 's': 'I',
    'zh': 'I', 'ch': 'I', 'sh': 'I', 'r': 'I',
    'y': 'U', 'w': 'U',
    'ai': 'A', 'ei': 'A', 'ao': 'O', 'ou': 'O',
    'an': 'E', 'en': 'E', 'in': 'E', 'un': 'E', 'ün': 'E',
    'ang': 'E', 'eng': 'E', 'ing': 'E', 'ong': 'E'
}

class LipSyncCore:
    def __init__(self, frame_rate=24.0, silence_threshold=0.01, max_silence_frames=5, language='chinese'):
        self.frame_rate = frame_rate
        self.silence_threshold = silence_threshold
        self.max_silence_frames = max_silence_frames
        self.set_language(language)
        logger.debug(f"LipSyncCore 初始化: frame_rate={frame_rate}, silence_threshold={silence_threshold}, max_silence_frames={max_silence_frames}, language={language}")

    def set_language(self, language):
        if language.lower() == 'english':
            self.phoneme_to_viseme = ENGLISH_PHONEME_TO_VISEME
        elif language.lower() == 'chinese':
            self.phoneme_to_viseme = CHINESE_PHONEME_TO_VISEME
        else:
            raise ValueError("Unsupported language. Choose 'english' or 'chinese'.")
        logger.info(f"设置语言为: {language}")

    def analyze_audio(self, audio_file):
        logger.info(f"开始分析音频文件: {audio_file}")
        y, sr = librosa.load(audio_file)
        duration = librosa.get_duration(y=y, sr=sr)
        logger.info(f"音频已加载. 采样率: {sr}, 音频时长: {duration} 秒")

        total_frames = int(duration * self.frame_rate)
        logger.debug(f"总帧数: {total_frames}")

        mel_spec = librosa.feature.melspectrogram(y=y, sr=sr, n_mels=10, hop_length=int(sr/self.frame_rate))
        chroma = librosa.feature.chroma_stft(y=y, sr=sr, hop_length=int(sr/self.frame_rate))
        logger.debug(f"生成 Mel 频谱图和色度图. Mel 频谱图形状: {mel_spec.shape}, 色度图形状: {chroma.shape}")

        visemes = self.generate_visemes(total_frames, mel_spec, chroma)
        logger.info(f"生成了 {len(visemes)} 个口型数据点")
        return visemes

    def generate_visemes(self, total_frames, mel_spec, chroma):
        logger.debug("开始生成口型序列")
        visemes = []
        silence_counter = 0
        current_viseme = None

        for frame in range(total_frames):
            time = frame / self.frame_rate
            frame_energy = np.sum(mel_spec[:, frame])

            if frame_energy > self.silence_threshold:
                pitch_frame = np.argmax(mel_spec[:, frame])
                chroma_frame = np.argmax(chroma[:, frame])
                phoneme_index = (pitch_frame + chroma_frame) % len(self.phoneme_to_viseme)
                current_phoneme = list(self.phoneme_to_viseme.keys())[phoneme_index]
                current_viseme = self.phoneme_to_viseme[current_phoneme]
                silence_counter = 0
            elif silence_counter >= self.max_silence_frames:
                current_viseme = None
            else:
                silence_counter += 1

            viseme_strength = min(1.0, frame_energy / self.silence_threshold) if current_viseme else 0.0
            visemes.append((frame, current_viseme, viseme_strength))
            if frame % 100 == 0:  # 每100帧打印一次，避免日志过多
                logger.debug(f"帧 {frame}: 时间 {time:.2f}s, 口型 {current_viseme}, 强度 {viseme_strength:.2f}, 能量 {frame_energy:.4f}, 静音计数 {silence_counter}")

        logger.debug("口型序列生成完成")
        return visemes

    def apply_visemes_to_mesh(self, obj, visemes, action_name):
        logger.info(f"开始将口型应用到网格, 对象: {obj.name}, 动作名称: {action_name}")
        if not obj.data.shape_keys:
            sk_basis = obj.shape_key_add(name='Basis')
            obj.data.shape_keys.use_relative = True
            logger.info("创建了'Basis'形态键")

        shape_keys = obj.data.shape_keys.key_blocks
        for viseme in ['A', 'I', 'U', 'E', 'O']:
            if viseme not in shape_keys:
                obj.shape_key_add(name=viseme)
                logger.info(f"创建了形态键: {viseme}")

        lip_sync_action = bpy.data.actions.new(name=action_name)
        if not obj.data.shape_keys.animation_data:
            obj.data.shape_keys.animation_data_create()
        obj.data.shape_keys.animation_data.action = lip_sync_action
        logger.debug(f"创建了新的动作: {action_name}")

        for frame, viseme, strength in visemes:
            for shape_key in shape_keys:
                if shape_key.name in ['A', 'I', 'U', 'E', 'O']:
                    shape_key.value = strength if shape_key.name == viseme else 0.0
                    shape_key.keyframe_insert("value", frame=frame)

        logger.info("完成将口型应用到网格")
        return lip_sync_action

    def create_nla_track(self, obj, action, track_name, strip_name):
        logger.info(f"开始创建NLA轨道, 对象: {obj.name}, 轨道名称: {track_name}, 条带名称: {strip_name}")
        if not obj.animation_data:
            obj.animation_data_create()
        
        nla_tracks = obj.animation_data.nla_tracks
        
        lip_sync_track = nla_tracks.new()
        lip_sync_track.name = track_name
        logger.debug(f"创建了新的NLA轨道: {track_name}")
        
        lip_sync_strip = lip_sync_track.strips.new(strip_name, start=1, action=action)
        lip_sync_strip.blend_type = 'REPLACE'
        lip_sync_strip.use_auto_blend = False
        lip_sync_strip.influence = 1.0
        logger.debug(f"在轨道 {track_name} 上创建了新的条带: {strip_name}")
        
        logger.info("完成NLA轨道创建")

    def process_audio_and_apply_lipsync(self, obj, audio_file):
        logger.info(f"开始处理音频和应用唇形同步, 对象: {obj.name}, 音频文件: {audio_file}")
        try:
            timestamp = int(time.time())
            action_name = f"LipSyncAction_{timestamp}"
            track_name = f"LipSync_{timestamp}"
            strip_name = f"LipSync_{timestamp}"

            visemes = self.analyze_audio(audio_file)
            action = self.apply_visemes_to_mesh(obj, visemes, action_name)
            self.create_nla_track(obj, action, track_name, strip_name)
            logger.info("完成音频处理和唇形同步应用")
        except Exception as e:
            logger.error(f"处理音频和应用唇形同步时发生错误: {str(e)}")
            raise

```

## Source file: lipsync_cleaner.py

{
  "path": "lipsync_cleaner.py",
  "blob_sha": "8d6e258905c0d9235b54b26264624f2cf5bcd8e9",
  "sha256": "983444c63dc32fe0a47cc66769c3cc424011fe425b0272a3581ba0655c4ca38b",
  "lines": 41,
  "bytes": 1840,
  "selection_score": 15,
  "keyword_line_counts": {
    "lip": 9
  },
  "url": "https://github.com/dukeren/lip_sync_3D_digital_human_for_Blender/blob/98ee84664dee43b94428a4520f3305e447fb31dc/lipsync_cleaner.py",
  "function_signature_candidates": [
    {
      "line": 6,
      "text": "def __init__(self, scene, lipsync_prefix, extra_frames, min_animation_frames):"
    },
    {
      "line": 11,
      "text": "def clear_lipsync_animation(self):"
    },
    {
      "line": 40,
      "text": "def threaded_adjust_scene_frame_range(self):"
    }
  ]
}

```text

import bpy
import threading
from frame_range_adjuster import FrameRangeAdjuster

class LipSyncCleaner:
    def __init__(self, scene, lipsync_prefix, extra_frames, min_animation_frames):
        self.scene = scene
        self.LIPSYNC_PREFIX = lipsync_prefix
        self.frame_range_adjuster = FrameRangeAdjuster(self.scene, extra_frames, min_animation_frames)

    def clear_lipsync_animation(self):
        obj = self.scene.lipsync_object
        if not obj:
            return

        if obj.animation_data:
            for track in [track for track in obj.animation_data.nla_tracks if self.LIPSYNC_PREFIX in track.name]:
                obj.animation_data.nla_tracks.remove(track)
            if obj.animation_data.action and self.LIPSYNC_PREFIX in obj.animation_data.action.name:
                obj.animation_data.action = None

        if obj.data.shape_keys and obj.data.shape_keys.animation_data:
            if obj.data.shape_keys.animation_data.action and self.LIPSYNC_PREFIX in obj.data.shape_keys.animation_data.action.name:
                obj.data.shape_keys.animation_data.action = None

        if self.scene.sequence_editor:
            for seq in [seq for seq in self.scene.sequence_editor.sequences if seq.type == 'SOUND' and seq.name != "Background Music"]:
                self.scene.sequence_editor.sequences.remove(seq)

        for action in bpy.data.actions:
            if self.LIPSYNC_PREFIX in action.name and action.users == 0:
                bpy.data.actions.remove(action)
        
        for sound in bpy.data.sounds:
            if sound.users == 0:
                bpy.data.sounds.remove(sound)

        threading.Thread(target=self.threaded_adjust_scene_frame_range).start()

    def threaded_adjust_scene_frame_range(self):
        new_end_frame = self.frame_range_adjuster.adjust_scene_frame_range()

```

## Source file: lipsync_animation_handler.py

{
  "path": "lipsync_animation_handler.py",
  "blob_sha": "8a52812e36428dba9d1ec82d9934136afab19425",
  "sha256": "e9fa37aa5fbaf2ae90b5937907c73c147c0654578732bc06b72f81b272bd9232",
  "lines": 193,
  "bytes": 8425,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 1,
    "lip": 39
  },
  "url": "https://github.com/dukeren/lip_sync_3D_digital_human_for_Blender/blob/98ee84664dee43b94428a4520f3305e447fb31dc/lipsync_animation_handler.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def __init__(self, scene):"
    },
    {
      "line": 17,
      "text": "def mark_lipsync_animation(self):"
    },
    {
      "line": 37,
      "text": "def check_new_lipsync_animation(self):"
    },
    {
      "line": 82,
      "text": "def get_next_available_start_frame(self):"
    },
    {
      "line": 88,
      "text": "def handle_new_lipsync_animation(self):"
    },
    {
      "line": 136,
      "text": "def apply_next_animation(self):"
    },
    {
      "line": 178,
      "text": "def is_lipsync_animation_finished(self):"
    },
    {
      "line": 191,
      "text": "def clear_animations(self):"
    }
  ]
}

```text

import bpy
from collections import deque
from .logger import get_logger
import time

class LipSyncAnimationHandler:
    LIPSYNC_PREFIX = "LipSync"
    EXTRA_FRAMES = 10
    MAX_FRAME = 100000

    def __init__(self, scene):
        self.scene = scene
        self.logger = get_logger()
        self.animation_queue = deque()
        self.current_animation_end_frame = self.scene.frame_start

    def mark_lipsync_animation(self):
        obj = self.scene.lipsync_object
        if not obj:
            return

        self.scene.lipsync_last_modified = time.time()
        
        self.scene.lipsync_action_name = obj.animation_data.action.name if obj.animation_data and obj.animation_data.action else ""
        self.scene.lipsync_track_count = len([track for track in obj.animation_data.nla_tracks if self.LIPSYNC_PREFIX in track.name]) if obj.animation_data else 0
        
        self.scene.lipsync_shape_key_action_name = (obj.data.shape_keys.animation_data.action.name 
                                               if obj.data.shape_keys and obj.data.shape_keys.animation_data and obj.data.shape_keys.animation_data.action 
                                               else "")
        
        if self.scene.sequence_editor:
            audio_sequences = [seq.name for seq in self.scene.sequence_editor.sequences if seq.type == 'SOUND' and seq.name != "Background Music"]
            self.scene.lipsync_audio_sequences = ",".join(audio_sequences)
        else:
            self.scene.lipsync_audio_sequences = ""

    def check_new_lipsync_animation(self):
        obj = self.scene.lipsync_object
        if not obj:
            return False

        current_time = time.time()
        if current_time - self.scene.lipsync_last_modified < 1.0:
            return False

        current_action_name = ""
        if obj.animation_data and obj.animation_data.action:
            if self.LIPSYNC_PREFIX in obj.animation_data.action.name:
                current_action_name = obj.animation_data.action.name

        current_track_count = 0
        if obj.animation_data:
            current_track_count = len([track for track in obj.animation_data.nla_tracks if self.LIPSYNC_PREFIX in track.name])

        current_shape_key_action_name = ""
        if obj.data.shape_keys and obj.data.shape_keys.animation_data and obj.data.shape_keys.animation_data.action:
            if self.LIPSYNC_PREFIX in obj.data.shape_keys.animation_data.action.name:
                current_shape_key_action_name = obj.data.shape_keys.animation_data.action.name

        has_lipsync_animation = (current_action_name != "" or 
                                current_track_count > 0 or 
                                current_shape_key_action_name != "")

        if not has_lipsync_animation:
            return False

        has_changes = (current_action_name != self.scene.lipsync_action_name or
                    current_track_count != self.scene.lipsync_track_count or
                    current_shape_key_action_name != self.scene.lipsync_shape_key_action_name)

        if has_changes:
            if current_action_name != self.scene.lipsync_action_name:
                self.logger.info(f"检测到新动画: action变化 {self.scene.lipsync_action_name} -> {current_action_name}")
            if current_track_count != self.scene.lipsync_track_count:
                self.logger.info(f"检测到新动画: track数量变化 {self.scene.lipsync_track_count} -> {current_track_count}")
            if current_shape_key_action_name != self.scene.lipsync_shape_key_action_name:
                self.logger.info(f"检测到新动画: shape key action变化 {self.scene.lipsync_shape_key_action_name} -> {current_shape_key_action_name}")
            return True

        return False

    def get_next_available_start_frame(self):
        if not self.animation_queue:
            return self.scene.frame_current
        last_animation = self.animation_queue[-1]
        return max(self.scene.frame_current, last_animation['start_frame'] + last_animation['duration'])

    def handle_new_lipsync_animation(self):
        self.logger.info("开始处理新的唇形动画")
        self.mark_lipsync_animation()
        
        obj = self.scene.lipsync_object
        if not obj:
            self.logger.warning("没有找到唇形同步对象")
            return

        if not obj.animation_data:
            obj.animation_data_create()
        
        new_action = bpy.data.actions.get(self.scene.lipsync_action_name)
        audio_sequences = self.scene.lipsync_audio_sequences.split(",") if self.scene.lipsync_audio_sequences else []
        
        duration = 0
        if new_action:
            duration = max(duration, int(new_action.frame_range[1] - new_action.frame_range[0]))
        
        if self.scene.sequence_editor:
            for seq_name in audio_sequences:
                seq = self.scene.sequence_editor.sequences.get(seq_name)
                if seq and seq.type == 'SOUND':
                    duration = max(duration, seq.frame_duration)
        
        duration += self.EXTRA_FRAMES
        
        start_frame = max(
            self.scene.frame_current,
            self.current_animation_end_frame,
            self.get_next_available_start_frame()
        )
        
        self.logger.info(f"处理新的唇形同步动画, 选定起始帧:{start_frame}, 持续时间:{duration}")
        
        new_animation = {
            'action_name': self.scene.lipsync_action_name,
            'shape_key_action_name': self.scene.lipsync_shape_key_action_name,
            'audio_sequences': audio_sequences,
            'start_frame': start_frame,
            'duration': duration
        }
        
        self.animation_queue.append(new_animation)
        self.logger.info(f"新动画添加到队列,队列长度:{len(self.animation_queue)}")
        
        return new_animation

    def apply_next_animation(self):
        if not self.animation_queue:
            return None

        animation = self.animation_queue.popleft()
        obj = self.scene.lipsync_object
        if not obj:
            return None

        start_frame = animation['start_frame']
        self.logger.info(f"应用新动画,开始帧:{start_frame}")
        max_end_frame = start_frame + animation['duration']

        new_action = bpy.data.actions.get(animation['action_name'])
        if new_action:
            track = obj.animation_data.nla_tracks.new()
            track.name = f"{self.LIPSYNC_PREFIX}_{len(obj.animation_data.nla_tracks)}"
            
            strip = track.strips.new(name=new_action.name, start=start_frame, action=new_action)
            strip.frame_end = start_frame + animation['duration']
        
        if obj.data.shape_keys and obj.data.shape_keys.animation_data:
            shape_key_action = bpy.data.actions.get(animation['shape_key_action_name'])
            if shape_key_action:
                obj.data.shape_keys.animation_data.action = shape_key_action
                for fcurve in shape_key_action.fcurves:
                    for keyframe in fcurve.keyframe_points:
                        keyframe.co.x += start_frame
                        keyframe.handle_left.x += start_frame
                        keyframe.handle_right.x += start_frame

        if self.scene.sequence_editor:
            for seq_name in animation['audio_sequences']:
                seq = self.scene.sequence_editor.sequences.get(seq_name)
                if seq and seq.type == 'SOUND':
                    seq.frame_start = start_frame

        self.current_animation_end_frame = max_end_frame
        self.logger.info(f"新动画应用完成,结束帧:{self.current_animation_end_frame}")

        return max_end_frame

    def is_lipsync_animation_finished(self):
        obj = self.scene.lipsync_object
        if not obj or not obj.animation_data:
            return True

        current_frame = self.scene.frame_current
        for track in obj.animation_data.nla_tracks:
            if self.LIPSYNC_PREFIX in track.name:
                for strip in track.strips:
                    if strip.frame_start <= current_frame < strip.frame_end:
                        return False
        return current_frame >= self.current_animation_end_frame

    def clear_animations(self):
        self.animation_queue.clear()
        self.current_animation_end_frame = self.scene.frame_start

```
