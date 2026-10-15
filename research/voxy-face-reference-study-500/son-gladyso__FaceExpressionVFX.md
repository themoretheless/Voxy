# Facial rendering source scan: son-gladyso/FaceExpressionVFX

Repository: https://github.com/son-gladyso/FaceExpressionVFX

Pinned source revision: 29f7d4bfb2d687ac80541efd39537e61ad5b6e9c

Discovery topic: FACS face

Description: Quality-gated FACS-inspired facial action-unit visualization with per-AU uncertainty and local-only processing.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"face": ["face-au-detector.js", "face-config.js", "face-detector.js", "face-export.js", "face-helpers.js", "face-history.js", "face-i18n.js", "face-model-adapter.js", "face-observables.js", "face-policies.js", "face-quality.js", "face-temporal.js"]}

## Source file: face-i18n.js

{
  "path": "face-i18n.js",
  "blob_sha": "462a56f97d7c7b7c16a11839a00bda239ea60ad6",
  "sha256": "f4b69615b537c4afd8b334040b4d92b7a5c5ee4984b5060bf06166c317e51f7c",
  "lines": 571,
  "bytes": 29292,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 5,
    "skin": 4,
    "sss": 4,
    "mouth": 2,
    "wrinkle": 2,
    "facs": 2,
    "rig": 4,
    "face": 12,
    "lip": 11
  },
  "url": "https://github.com/son-gladyso/FaceExpressionVFX/blob/29f7d4bfb2d687ac80541efd39537e61ad5b6e9c/face-i18n.js",
  "function_signature_candidates": []
}

```text

/* ═══════════════════════════════════════════════════════════════
 *  face-i18n.js  –  Internationalization Module
 *  Supports: zh-CN (default), en-US
 *  Usage:  FaceI18n.t('key')  /  FaceI18n.setLocale('en-US')
 * ═══════════════════════════════════════════════════════════════ */
(function (global) {
  'use strict';

  var _locale = 'zh-CN';
  var _listeners = [];

  // Restore persisted locale
  try {
    var stored = (typeof localStorage !== 'undefined') && localStorage.getItem('faceexpr_locale');
    if (stored && (stored === 'zh-CN' || stored === 'en-US')) _locale = stored;
  } catch (e) { void e; }

  /* ═══ Translation Dictionaries ═══ */
  var DICT = {
    'zh-CN': {
      /* ── Page / Titles ── */
      pageTitle:           'FaceVFX v3.11 — 可观察面部动作分析',
      cameraTitle:         '面部深度扫描 (78 Key Landmarks)',
      exprTitle:           '最强 AU 激活与置信区间',
      authTitle:           '贝叶斯时序 AU 状态',
      auTitle:             'FACS 动作单元拓扑',
      psyTitle:            '可观察动作摘要 (12D)',
      transTitle:          '逐帧质量标记',
      analysisTitle:       '质量、AU 区间与来源',
      microTitle:          '校准与模型状态',
      headPoseTitle:       '头部姿态阵列',

      /* ── Status ── */
      statusScanning:      '面部扫描中...',
      statusPaused:        '已暂停 (标签页不可见)',
      statusOnline:        '系统在线 — 深度分析中',
      statusQualityPaused: '当前帧质量不足，AU 输出已暂停',
      statusNoFace:        '未检测到面部',
      statusInit:          '系统神经网络初始化中...',
      statusNeedHTTPS:     '需要 HTTPS 或 localhost 环境',
      runtimeError:        '运行时错误，请刷新页面重试。',

      /* ── Camera Errors ── */
      cameraUnknown:       '未知摄像头错误',
      cameraNotAllowed:    '摄像头权限被拒绝，请在浏览器设置中允许摄像头访问。',
      cameraNotFound:      '未检测到摄像头设备。',
      cameraNotReadable:   '摄像头被其他程序占用，请关闭后重试。',
      cameraError:         '摄像头错误: ',

      /* ── Authenticity ── */
      authDuchenne:        '关键点完整性',
      authTemporal:        '后验确定度',
      authSymmetry:        '姿态适用度',
      authDuration:        '状态持续时间',
      authConflict:        '光照质量',
      authLeakage:         '图像清晰度',
      calibrationLocked:   '个人基线已锁定',
      calibrationCollecting:'正在采集稳健个人基线',

      /* ── Expression Labels (29) ── */
      'expr.genuine_smile':    '真笑 (Duchenne)',
      'expr.social_smile':     '社交微笑',
      'expr.suppressed_smile': '抑制微笑',
      'expr.surprise':         '惊讶',
      'expr.fear':             '恐惧',
      'expr.anger':            '愤怒',
      'expr.disgust':          '厌恶',
      'expr.contempt':         '蔑视',
      'expr.sadness':          '悲伤',
      'expr.anxiety':          '焦虑',
      'expr.tension':          '紧张',
      'expr.concentration':    '专注',
      'expr.confusion':        '困惑',
      'expr.disdain':          '不屑',
      'expr.embarrassment':    '尴尬',
      'expr.pain':             '痛苦',
      'expr.fatigue':          '疲倦',
      'expr.boredom':          '厌烦',
      'expr.interest':         '兴趣/好奇',
      'expr.pleasure':         '愉悦',
      'expr.suppression':      '情绪抑制',
      'expr.au12_au14_asymmetry_pattern': 'AU12/AU14 不对称模式',
      'expr.masked_sadness':   '掩饰悲伤',
      'expr.nervous_smile':    '紧张微笑',
      'expr.irritation':         '烦躁',
      'expr.melancholy':         '忧郁',
      'expr.unease':             '不安',
      'expr.distaste':           '反感',
      'expr.neutral':          '中性',

      /* ── Psychology Labels (12) ── */
      'psy.valence':             '情绪效价',
      'psy.arousal':             '唤醒度',
      'psy.cognitiveLoad':       '认知负荷',
      'psy.stress':              '压力指数',
      'psy.confidence':          '自信度',
      'psy.engagement':          '专注度',
      'psy.comfort':             '舒适度',
      'psy.authenticity':        '表情一致度',
      'psy.tension':             '紧张度',
      'psy.patternAnomaly':      '模式异常',
      'psy.suppressionEffort':   '抑制努力',
      'psy.emotionalRegulation': '情绪调节力',

      /* ── Observable movement labels (v3.11) ── */
      'obs.browRaise':            '眉部上提',
      'obs.browLower':            '眉部下压',
      'obs.eyeOpening':           '眼睑张开',
      'obs.eyeClosure':           '眼睑闭合',
      'obs.cheekRaise':           '面颊上提',
      'obs.noseWrinkle':          '鼻部皱缩',
      'obs.lipCornerPull':        '嘴角上提',
      'obs.lipCornerDepress':     '嘴角下压',
      'obs.mouthOpening':         '口部张开',
      'obs.lipCompression':       '唇部压紧',
      'obs.bilateralAsymmetry':   '双侧不对称度',
      'obs.motionEnergy':         '动作变化量',

      /* ── UI Strings ── */
      blink:               '眨眼',
      leakage:             '泄露',
      /* ── Punctuation (locale-aware) ── */
      'p.lp': '（', 'p.rp': '）', 'p.comma': '，', 'p.semi': '；',

      /* ── Analysis Text Strings ── */
      'analysis.currentExpr':     '【当前表情】',
      'analysis.confidence':      '置信度',
      'analysis.authenticity':    '表情一致性',
      'analysis.authReal':        '高',
      'analysis.authSuspect':     '中',
      'analysis.authFake':        '低',
      'analysis.authWarning':     '⚠ 低一致性提示: ',
      'analysis.noDuchenne':      '缺少杜兴标记(眼轮匝肌不参与)',
      'analysis.temporalAnomaly': '时序特征异常',
      'analysis.asymmetry':       '面部不对称',
      'analysis.muscleConflict':  '检测到肌肉冲突',
      'analysis.leakageDetected': '检测到短时冲突模式',
      'analysis.dissonance':      '【模式冲突】',
      'analysis.dissonanceHigh':  '显著',
      'analysis.dissonanceMild':  '轻度',
      'analysis.transition':      '【表情转换】',
      'analysis.psyState':        '【实验性状态指标】',
      'analysis.negativeValence':  '情绪偏负面',
      'analysis.positiveValence':  '情绪积极',
      'analysis.highArousal':     '情绪唤醒度较高',
      'analysis.highCogLoad':     '认知负荷代理指标较高',
      'analysis.stressSign':      '压力代理指标较高',
      'analysis.facialTension':   '面部肌肉紧张',
      'analysis.deceptionSignal': '异常模式分数较高',
      'analysis.suppressing':     '抑制模式分数较高',
      'analysis.lowComfort':      '舒适度代理指标较低',
      'analysis.suppression':     '【抑制模式指标】',
      'analysis.suppressStrong':  '强烈',
      'analysis.suppressMid':     '中度',
      'analysis.suppressWord':    '抑制',
      'analysis.lipTight':        '嘴唇紧抿',
      'analysis.browSmile':       '皱眉同时微笑',
      'analysis.antagonist':      '对抗性肌肉活动',
      'analysis.regulation':      '【调节模式】',
      'analysis.inertia':         '【情绪惯性】',
      'analysis.inertiaLow':      '低 — 情绪变化频繁，可能处于不稳定状态',
      'analysis.inertiaHigh':     '高 — 情绪状态稳定持久',
      'analysis.baselineDeviation': '【基线偏离】',
      'analysis.microExpr':       '【微表情】',
      'analysis.headPose':        '【头部姿态】',
      'analysis.headLeft':        '头偏左',
      'analysis.headRight':       '头偏右',
      'analysis.headDown':        '低头',
      'analysis.headUp':          '抬头',
      'analysis.headTilt':        '头部倾斜',
      'analysis.blinkAnalysis':   '【眨眼分析】',
      'analysis.blinkPerMin':     '次/分',
      'analysis.blinkVeryHigh':   '异常偏高 — 强烈压力/焦虑信号',
      'analysis.blinkHigh':       '偏高 — 可能存在压力或认知负荷',
      'analysis.blinkSlightHigh': '略高 — 轻度唤醒',
      'analysis.blinkNormal':     '正常范围',
      'analysis.blinkLow':        '偏低 — 高度专注或抑制状态',
      'analysis.perclosExceed':   '⚠ 超过困倦阈值',
      'analysis.perclosNear':     '接近阈值',
      'analysis.perclosRef':      '(Dinges 1998: >15%=困倦)',
      'analysis.blinkCluster':    '检测到眨眼聚簇',
      'analysis.blinkClusterNote':'组) — 间歇性认知负荷波动',
      'analysis.ibiHigh':         '眨眼间隔变异系数高',
      'analysis.ibiNote':         ' — 情绪波动或注意力不稳定',
      'analysis.smileBrowFurrow': '微笑时眉头紧锁',
      'analysis.smileLipPress':   '微笑时嘴唇紧闭',
      'analysis.calmButTense':    '表面平静但面部肌肉紧张',
      'analysis.negativeSmileLeak':  '负面表情中检测到笑容AU泄漏，情绪表达可能并不真实',
      'analysis.sadnessWithCheekRaise': '悲伤伴随颔骨提升(AU6)，可能为刻意布置的悲伤表情',
      'analysis.fearWithControl':    '恐惧伴随下巴抬升+唇紧压，显示主动控制，可能非真实恐惧',
      'analysis.smileAsymmetric': '笑容不对称',
      'analysis.antagonistMuscle':'拮抗肌群同时激活',

      /* ── Transition Significance ── */
      'trans.genuineToNeutralSmooth': '自然的情绪回落，表明情绪体验真实',
      'trans.socialToNeutralAbrupt':  '社交面具突然卸下，可能的疲倦或不耐烦',
      'trans.neutralToSmileAbrupt':   '快速切换至笑容，可能是刻意表演',
      'trans.genuineToContempt':      '真笑转蔑视：优越感显露，暗示态度转变',
      'trans.socialToContempt':       '伪装友好后露出轻蔑，真实情绪泄露',
      'trans.angerToSocial':          '愤怒后强装微笑，情绪调节/社会掩饰',
      'trans.sadnessToGenuine':       '悲伤转真笑：情绪恢复或释然',
      'trans.sadnessToSocial':        '悲伤后社交微笑：试图掩饰内心痛苦',
      'trans.suppressionToSmile':     '抑制后微笑：压力释放或防御机制',
      'trans.fearToSocial':           '恐惧转社交笑：典型的恐惧掩饰行为',
      'trans.toSuppression':          '进入情绪抑制状态：主动控制情绪表达',
      'trans.concToSurprise':         '专注中被打断：注意力突然转移',
      'trans.neutralToAnxiety':       '从平静进入焦虑：可能感知到威胁或压力',
      'trans.neutralToAnger':         '平静转为愤怒：突发性情绪触发，可能的挫折激发',
      'trans.neutralToSadness':       '平静转为悲伤：可能回想起伤痛记忆或感受到失落',
      'trans.neutralToDisgust':       '平静转为双恶：对某事物产生本能反感',
      'trans.fearToAnger':            '恐惧转为愤怒：战或逃反应中的“战斗”激活',
      'trans.sadnessToAnger':         '悲伤转为愤怒：哀伤引发愤怒，常见于悲伤的中期阶段',
      'trans.angerToSadness':         '愤怒转为悲伤：愤怒消退后的后悔或无力感',
      'trans.suppressedType':         '转换过程中检测到肌肉冲突，情绪表达受到抑制',
      'trans.abruptNonNeutral':       '情绪间快速跳转，可能存在情绪不稳定或刻意切换',

      /* ── Blink Interpret Notes ── */
      'blink.drowsy':     'PERCLOS={0}%，眼睛长时间闭合，可能困倦或疲劳',
      'blink.elevated':   '眨眼频率偏高(>{0}/min)，可能有压力或焦虑',
      'blink.reduced':    '眨眼频率偏低(<{0}/min)，可能正在专注或疲劳',
      'blink.irregular':  '眨眼节律不规则(CV={0})，可能认知负荷波动',

      /* ── Regulation Styles ── */
      'reg.undetermined':  '未确定',
      'reg.suppressive':   '压抑型 (Response-focused)',
      'reg.adaptive':      '适应型 (Adaptive)',
      'reg.masking':       '伪装型 (Masking)',
      'reg.expressive':    '自然表达型 (Expressive)',
      'reg.mixed':         '混合型 (Mixed)',

      /* ── Toolbar / Export / History ── */
      langLabel:           '语言',
      exportJSON:          '导出 JSON',
      exportCSV:           '导出 CSV',
      exportTitle:         '数据导出',
      historyTitle:        '历史记录',
      historyOpen:         '历史',
      historyProfile:      '档案',
      historyClear:        '清除记录',
      historyClearAll:     '清除全部',
      historyNoData:       '暂无历史记录',
      historySession:      '会话',
      historyDuration:     '时长',
      historyFrames:       '帧数',
      historyAvg:          '平均',
      historyTrend:        '趋势',
      historyTrendUp:      '↑上升',
      historyTrendDown:    '↓下降',
      historyTrendStable:  '→稳定',
      historyNewProfile:   '新建档案',
      historyDefaultName:  '默认用户',
      historyDeleteProfile:'删除档案',
      historyConfirmDelete:'确定删除此档案及所有记录吗？',
      historyConfirmClear: '确定清除所有历史记录吗？',
      historyExport:       '导出历史',
      historySaving:       '保存中...',
      historySaved:        '已保存',
      historyStart:        '开始记录',
      historyStop:         '停止记录',
      historyRecording:    '● 记录中',
      historyUnavailable:  '当前环境不支持历史记录',
      installApp:          '⬇ 安装应用',
    },

    'en-US': {
      /* ── Page / Titles ── */
      pageTitle:           'FaceVFX v3.11 — Observable Facial Movement Analysis',
      cameraTitle:         'Face Deep Scan (78 Key Landmarks)',
      exprTitle:           'Strongest AU Activations & Intervals',
      authTitle:           'Bayesian Temporal AU State',
      auTitle:             'FACS Action Unit Topology',
      psyTitle:            'Observable Movement Summary (12D)',
      transTitle:          'Frame Quality Flags',
      analysisTitle:       'Quality, AU Intervals & Provenance',
      microTitle:          'Calibration & Model Status',
      headPoseTitle:       'Head Pose Array',

      /* ── Status ── */
      statusScanning:      'Scanning face...',
      statusPaused:        'Paused (tab hidden)',
      statusOnline:        'System online — deep analysis',
      statusQualityPaused: 'Frame quality insufficient; AU output paused',
      statusNoFace:        'No face detected',
      statusInit:          'Neural network initializing...',
      statusNeedHTTPS:     'HTTPS or localhost required',
      runtimeError:        'Runtime error, please refresh.',

      /* ── Camera Errors ── */
      cameraUnknown:       'Unknown camera error',
      cameraNotAllowed:    'Camera access denied. Please allow in browser settings.',
      cameraNotFound:      'No camera device found.',
      cameraNotReadable:   'Camera occupied by another app. Close it and retry.',
      cameraError:         'Camera error: ',

      /* ── Authenticity ── */
      authDuchenne:        'Landmark Integrity',
      authTemporal:        'Posterior Certainty',
      authSymmetry:        'Pose Suitability',
      authDuration:        'State Duration',
      authConflict:        'Lighting Quality',
      authLeakage:         'Sharpness',
      calibrationLocked:   'Personal baseline locked',
      calibrationCollecting:'Collecting robust personal baseline',

      /* ── Expression Labels (29) ── */
      'expr.genuine_smile':    'Genuine Smile (Duchenne)',
      'expr.social_smile':     'Social Smile',
      'expr.suppressed_smile': 'Suppressed Smile',
      'expr.surprise':         'Surprise',
      'expr.fear':             'Fear',
      'expr.anger':            'Anger',
      'expr.disgust':          'Disgust',
      'expr.contempt':         'Contempt',
      'expr.sadness':          'Sadness',
      'expr.anxiety':          'Anxiety',
      'expr.tension':          'Tension',
      'expr.concentration':    'Concentration',
      'expr.confusion':        'Confusion',
      'expr.disdain':          'Disdain',
      'expr.embarrassment':    'Embarrassment',
      'expr.pain':             'Pain',
      'expr.fatigue':          'Fatigue',
      'expr.boredom':          'Boredom',
      'expr.interest':         'Interest / Curiosity',
      'expr.pleasure':         'Pleasure',
      'expr.suppression':      'Emotion Suppression',
      'expr.au12_au14_asymmetry_pattern': 'AU12/AU14 Asymmetry Pattern',
      'expr.masked_sadness':   'Masked Sadness',
      'expr.nervous_smile':    'Nervous Smile',
      'expr.irritation':         'Irritation',
      'expr.melancholy':         'Melancholy',
      'expr.unease':             'Unease',
      'expr.distaste':           'Distaste',
      'expr.neutral':          'Neutral',

      /* ── Psychology Labels (12) ── */
      'psy.valence':             'Valence',
      'psy.arousal':             'Arousal',
      'psy.cognitiveLoad':       'Cognitive Load',
      'psy.stress':              'Stress',
      'psy.confidence':          'Confidence',
      'psy.engagement':          'Engagement',
      'psy.comfort':             'Comfort',
      'psy.authenticity':        'Expression Consistency',
      'psy.tension':             'Tension',
      'psy.patternAnomaly':      'Pattern Anomaly',
      'psy.suppressionEffort':   'Suppression Effort',
      'psy.emotionalRegulation': 'Emotional Regulation',

      /* ── Observable movement labels (v3.11) ── */
      'obs.browRaise':            'Brow Raise',
      'obs.browLower':            'Brow Lower',
      'obs.eyeOpening':           'Eye Opening',
      'obs.eyeClosure':           'Eye Closure',
      'obs.cheekRaise':           'Cheek Raise',
      'obs.noseWrinkle':          'Nose Wrinkle',
      'obs.lipCornerPull':        'Lip Corner Pull',
      'obs.lipCornerDepress':     'Lip Corner Depress',
      'obs.mouthOpening':         'Mouth Opening',
      'obs.lipCompression':       'Lip Compression',
      'obs.bilateralAsymmetry':   'Bilateral Asymmetry',
      'obs.motionEnergy':         'Motion Energy',

      /* ── UI Strings ── */
      blink:               'Blink',
      leakage:             'Leak',
      /* ── Punctuation (locale-aware) ── */
      'p.lp': '(', 'p.rp': ')', 'p.comma': ', ', 'p.semi': '; ',

      /* ── Analysis Text Strings ── */
      'analysis.currentExpr':     '[Expression] ',
      'analysis.confidence':      'confidence',
      'analysis.authenticity':    'expression consistency',
      'analysis.authReal':        'high',
      'analysis.authSuspect':     'medium',
      'analysis.authFake':        'low',
      'analysis.authWarning':     '⚠ Low-consistency note: ',
      'analysis.noDuchenne':      'no Duchenne marker (orbicularis oculi inactive)',
      'analysis.temporalAnomaly': 'temporal anomaly',
      'analysis.asymmetry':       'facial asymmetry',
      'analysis.muscleConflict':  'muscle conflict detected',
      'analysis.leakageDetected': 'brief conflicting pattern detected',
      'analysis.dissonance':      '[Pattern Conflict] ',
      'analysis.dissonanceHigh':  'Significant',
      'analysis.dissonanceMild':  'Mild',
      'analysis.transition':      '[Expression Transition] ',
      'analysis.psyState':        '[Experimental State Indicators] ',
      'analysis.negativeValence':  'negative valence',
      'analysis.positiveValence':  'positive valence',
      'analysis.highArousal':     'high arousal',
      'analysis.highCogLoad':     'elevated cognitive-load proxy',
      'analysis.stressSign':      'elevated stress proxy',
      'analysis.facialTension':   'facial muscle tension',
      'analysis.deceptionSignal': 'elevated anomaly-pattern score',
      'analysis.suppressing':     'elevated suppression-pattern score',
      'analysis.lowComfort':      'low comfort proxy',
      'analysis.suppression':     '[Suppression-pattern indicator] ',
      'analysis.suppressStrong':  'Strong',
      'analysis.suppressMid':     'Moderate',
      'analysis.suppressWord':    'suppression',
      'analysis.lipTight':        'lips tightly pressed',
      'analysis.browSmile':       'frowning while smiling',
      'analysis.antagonist':      'antagonist muscle activity',
      'analysis.regulation':      '[Regulation Mode] ',
      'analysis.inertia':         '[Emotional Inertia] ',
      'analysis.inertiaLow':      'Low — frequent emotional shifts, possibly unstable',
      'analysis.inertiaHigh':     'High — stable emotional state',
      'analysis.baselineDeviation': '[Baseline Deviation] ',
      'analysis.microExpr':       '[Micro-expression] ',
      'analysis.headPose':        '[Head Pose] ',
      'analysis.headLeft':        'head left',
      'analysis.headRight':       'head right',
      'analysis.headDown':        'head down',
      'analysis.headUp':          'head up',
      'analysis.headTilt':        'head tilt',
      'analysis.blinkAnalysis':   '[Blink Analysis] ',
      'analysis.blinkPerMin':     '/min',
      'analysis.blinkVeryHigh':   'abnormally high — severe stress/anxiety',
      'analysis.blinkHigh':       'elevated — possible stress or cognitive load',
      'analysis.blinkSlightHigh': 'slightly high — mild arousal',
      'analysis.blinkNormal':     'normal range',
      'analysis.blinkLow':        'low — intense focus or suppression',
      'analysis.perclosExceed':   '⚠ exceeds drowsiness threshold',
      'analysis.perclosNear':     'approaching threshold',
      'analysis.perclosRef':      '(Dinges 1998: >15%=drowsy)',
      'analysis.blinkCluster':    'blink clustering detected',
      'analysis.blinkClusterNote':'groups) — intermittent cognitive load fluctuation',
      'analysis.ibiHigh':         'high IBI coefficient of variation',
      'analysis.ibiNote':         ' — emotional fluctuation or attention instability',
      'analysis.smileBrowFurrow': 'brow furrowed while smiling',
      'analysis.smileLipPress':   'lips pressed while smiling',
      'analysis.calmButTense':    'calm surface but facial muscles tense',
      'analysis.negativeSmileLeak':  'smile AU leak during negative expression — possibly not genuine',
      'analysis.sadnessWithCheekRaise': 'sadness with cheek raise (AU6) — possibly posed sadness',
      'analysis.fearWithControl':    'fear with chin raise + lip press — deliberate control, possibly not genuine fear',
      'analysis.smileAsymmetric': 'asymmetric smile',
      'analysis.antagonistMuscle':'antagonist muscles co-activated',

      /* ── Transition Significance ── */
      'trans.genuineToNeutralSmooth': 'Natural emotional decline, indicating genuine emotional experience',
      'trans.socialToNeutralAbrupt':  'Social mask abruptly dropped — possible fatigue or impatience',
      'trans.neutralToSmileAbrupt':   'Quick switch to smile — possibly deliberate performance',
      'trans.genuineToContempt':      'Genuine smile → contempt: superiority emerging, attitude shift',
      'trans.socialToContempt':       'Feigned friendliness → contempt: true emotion leaking',
      'trans.angerToSocial':          'Forced smile after anger — emotion regulation / social masking',
      'trans.sadnessToGenuine':       'Sadness → genuine smile: emotional recovery or relief',
      'trans.sadnessToSocial':        'Social smile after sadness: attempting to conceal inner pain',
      'trans.suppressionToSmile':     'Smile after suppression: stress release or defense mechanism',
      'trans.fearToSocial':           'Fear → social smile: typical fear-masking behavior',
      'trans.toSuppression':          'Entering suppression: actively controlling emotional expression',
      'trans.concToSurprise':         'Concentration interrupted: sudden attention shift',
      'trans.neutralToAnxiety':       'Calm → anxiety: perceived threat or pressure',
      'trans.neutralToAnger':           'Calm → anger: sudden frustration trigger',
      'trans.neutralToSadness':         'Calm → sadness: painful memory recall or sense of loss',
      'trans.neutralToDisgust':         'Calm → disgust: instinctive aversion response',
      'trans.fearToAnger':              'Fear → anger: fight-or-flight "fight" activation',
      'trans.sadnessToAnger':           'Sadness → anger: grief triggering rage (mid-stage grief)',
      'trans.angerToSadness':           'Anger → sadness: post-anger regret or helplessness',
      'trans.suppressedType':         'Muscle conflict during transition — expression suppressed',
      'trans.abruptNonNeutral':       'Rapid emotion shift — possible instability or deliberate switch',

      /* ── Blink Interpret Notes ── */
      'blink.drowsy':     'PERCLOS={0}% — prolonged eye closure, possible drowsiness or fatigue',
      'blink.elevated':   'Blink rate elevated (>{0}/min) — possible stress or anxiety',
      'blink.reduced':    'Blink rate reduced (<{0}/min) — possible deep focus or fatigue',
      'blink.irregular':  'Irregular blink rhythm (CV={0}) — cognitive load fluctuation',

      /* ── Regulation Styles ── */
      'reg.undetermined':  'Undetermined',
      'reg.suppressive':   'Suppressive (Response-focused)',
      'reg.adaptive':      'Adaptive',
      'reg.masking':       'Masking',
      'reg.expressive':    'Expressive',
      'reg.mixed':         'Mixed',

      /* ── Toolbar / Export / History ── */
      langLabel:           'Lang',
      exportJSON:          'Export JSON',
      exportCSV:           'Export CSV',
      exportTitle:         'Data Export',
      historyTitle:        'History',
      historyOpen:         'History',
      historyProfile:      'Profile',
      historyClear:        'Clear Records',
      historyClearAll:     'Clear All',
      historyNoData:       'No history data',
      historySession:      'Session',
      historyDuration:     'Duration',
      historyFrames:       'Frames',
      historyAvg:          'Avg',
      historyTrend:        'Trend',
      historyTrendUp:      '↑ rising',
      historyTrendDown:    '↓ falling',
      historyTrendStable:  '→ stable',
      historyNewProfile:   'New Profile',
      historyDefaultName:  'Default User',
      historyDeleteProfile:'Delete Profile',
      historyConfirmDelete:'Delete this profile and all records?',
      historyConfirmClear: 'Clear all history records?',
      historyExport:       'Export History',
      historySaving:       'Saving...',
      historySaved:        'Saved',
      historyStart:        'Start Recording',
      historyStop:         'Stop Recording',
      historyRecording:    '● Recording',
      historyUnavailable:  'History unavailable in this environment',
      installApp:          '⬇ Install',
    },
  };

  /* ═══ Translate Function ═══ */
  function t(key) {
    var dict = DICT[_locale] || DICT['zh-CN'];
    if (dict[key] !== undefined) return dict[key];
    // Fallback to zh-CN
    if (DICT['zh-CN'][key] !== undefined) return DICT['zh-CN'][key];
    return key;
  }

  /* ═══ Expression label helper (maps expression name → localized label) ═══ */
  function exprLabel(exprName) {
    return t('expr.' + exprName) || exprName;
  }

  /* ═══ Psychology label helper ═══ */
  function psyLabel(psyKey) {
    return t('psy.' + psyKey) || psyKey;
  }

  function observableLabel(key) {
    return t('obs.' + key) || key;
  }

  /* ═══ Get / Set Locale ═══ */
  function getLocale() { return _locale; }

  function setLocale(locale) {
    if (!DICT[locale]) return;
    _locale = locale;
    // Persist choice
    try { if (typeof localStorage !== 'undefined') localStorage.setItem('faceexpr_locale', locale); } catch (e) { void e; }
    for (var i = 0; i < _listeners.length; i++) {
      _listeners[i](locale);
    }
  }

  function getAvailableLocales() {
    return Object.keys(DICT);
  }

  /* ═══ Locale Change Listener ═══ */
  function onLocaleChange(fn) {
    _listeners.push(fn);
  }

  function removeLocaleListener(fn) {
    _listeners = _listeners.filter(function (f) { return f !== fn; });
  }

  global.FaceI18n = Object.freeze({
    t:                   t,
    exprLabel:           exprLabel,
    psyLabel:            psyLabel,
    observableLabel:     observableLabel,
    getLocale:           getLocale,
    setLocale:           setLocale,
    getAvailableLocales: getAvailableLocales,
    onLocaleChange:      onLocaleChange,
    removeLocaleListener:removeLocaleListener,
  });
})(typeof window !== 'undefined' ? window : globalThis);


```

## Source file: face-config.js

{
  "path": "face-config.js",
  "blob_sha": "cd02d7caa27ca7a9974dbbac6f4773c782375887",
  "sha256": "ead0edd9617178d7a704b5415e218226c63838ddea6eeb2317c9dba437565cba",
  "lines": 290,
  "bytes": 12153,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 17,
    "jaw": 2,
    "mouth": 13,
    "wrinkle": 3,
    "facs": 3,
    "iris": 3,
    "rig": 19,
    "face": 7,
    "lip": 11
  },
  "url": "https://github.com/son-gladyso/FaceExpressionVFX/blob/29f7d4bfb2d687ac80541efd39537e61ad5b6e9c/face-config.js",
  "function_signature_candidates": []
}

```text

/* ═══════════════════════════════════════════════════════════════
 *  face-config.js  –  FACS Action Unit & Micro-Expression Config
 *  v3 – Scientific Enhancement: 78 key landmarks, temporal
 *  dynamics, authenticity thresholds, transition analysis.
 *
 *  Scientific references:
 *    Ekman & Friesen (1978) – FACS coding manual
 *    Ekman (2003) – Duchenne vs non-Duchenne smile
 *    Hess & Kleck (1990) – Temporal dynamics of expressions
 *    Cohn & Schmidt (2004) – AU onset/offset analysis
 *    Ekman et al. (1981) – Facial asymmetry & posed emotion
 *    Porter & ten Brinke (2008) – Micro-expression leakage
 * ═══════════════════════════════════════════════════════════════ */
(function (global) {
  'use strict';

  var FACE_CONFIG = Object.freeze({

    /* ── MediaPipe Face Mesh Landmark Indices ── */
    LANDMARKS: Object.freeze({
      // Eyes – 6-point EAR [outer, upperA, upperB, inner, lowerB, lowerA]
      rightEye:   Object.freeze([33, 160, 158, 133, 153, 144]),
      leftEye:    Object.freeze([362, 385, 387, 263, 373, 380]),
      // Redundant eye points (occlusion-robust)
      rightEyeUpper: Object.freeze([246, 161, 160, 159, 158, 157, 173]),
      rightEyeLower: Object.freeze([7, 163, 144, 145, 153, 154, 155]),
      leftEyeUpper:  Object.freeze([466, 388, 387, 386, 385, 384, 398]),
      leftEyeLower:  Object.freeze([249, 390, 373, 374, 380, 381, 382]),
      // Iris (refined 468-477)
      rightIris: Object.freeze([468, 469, 470, 471, 472]),
      leftIris:  Object.freeze([473, 474, 475, 476, 477]),
      // ── NEW: Periorbital / Crow's feet (Orbicularis Oculi) ──
      // Critical for Duchenne marker detection (Ekman, 2003)
      rightPeriorbital: Object.freeze([130, 247, 30, 29]),
      leftPeriorbital:  Object.freeze([359, 467, 260, 259]),
      // Eyebrows – inner→outer
      rightBrow: Object.freeze([107, 66, 105, 63, 70]),
      leftBrow:  Object.freeze([336, 296, 334, 293, 300]),
      // ── NEW: Corrugator Supercilii (refined AU4) ──
      rightCorrugator: Object.freeze([46, 53]),
      leftCorrugator:  Object.freeze([276, 283]),
      // Nose
      noseBridge: 168, noseMid: 6, noseTip: 1, noseBase: 2,
      noseWing:   Object.freeze([48, 278]),
      // ── NEW: Nasolabial Fold (key for disgust/contempt) ──
      rightNasolabial: Object.freeze([49, 220]),
      leftNasolabial:  Object.freeze([279, 440]),
      // Mouth outer
      mouthLeft: 61, mouthRight: 291,
      mouthTopCenter: 0, mouthBottomCenter: 17,
      mouthOuter: Object.freeze([61,185,40,39,37,0,267,269,270,409,291,375,321,405,314,17,84,181]),
      // Mouth inner
      innerLipTop: 13, innerLipBottom: 14,
      innerMouthLeft: 78, innerMouthRight: 308,
      mouthInner: Object.freeze([78,191,80,81,82,13,312,311,310,415,308,324,318,402,317,14,87,178]),
      // ── NEW: Orbicularis Oris (refined lip) ──
      orbicularisOris: Object.freeze([87, 317, 164]),
      // ── NEW: Mentalis (chin region, AU17) ──
      mentalis: Object.freeze([199, 175, 18]),
      // Face reference anchors
      foreheadTop: 10, chin: 152, leftFace: 234, rightFace: 454,
      // ── NEW: Forehead midpoints ──
      foreheadMid: Object.freeze([151, 9]),
      // Cheek
      rightUpperCheek: 50, leftUpperCheek: 280,
      rightMidCheek: 205, leftMidCheek: 425,
      // ── NEW: Zygomatic major (cheekbone, key for AU6/AU12) ──
      rightZygomatic: 123, leftZygomatic: 352,
      // ── NEW: Temporalis / Masseter (jaw clenching) ──
      rightTemporalis: 116, leftTemporalis: 345,
      // Contour
      contour: Object.freeze([10,338,297,332,284,251,389,356,454,
        323,361,288,397,365,379,378,400,377,152,148,176,149,
        150,136,172,58,132,93,234,127,162,21,54,103,67,109]),
      // ── Head Pose Reference Points ──
      headPose: Object.freeze({
        noseTip: 1, chin: 152,
        leftEyeOuter: 263, rightEyeOuter: 33,
        leftMouth: 291, rightMouth: 61,
      }),
    }),

    /* ── 78 Key Display Landmarks (expanded from 50) ── */
    KEY_LANDMARKS_78: Object.freeze([
      // Right brow (5)
      107, 66, 105, 63, 70,
      // Left brow (5)
      336, 296, 334, 293, 300,
      // Right eye (8)
      33, 160, 159, 158, 133, 144, 145, 153,
      // Left eye (8)
      362, 385, 386, 387, 263, 380, 374, 373,
      // Nose (4)
      168, 1, 2, 4,
      // Mouth outer (8)
      61, 40, 0, 269, 291, 405, 17, 181,
      // Mouth inner (4)
      78, 13, 308, 14,
      // Face anchors (4)
      10, 152, 234, 454,
      // Cheeks (4)
      50, 280, 205, 425,
      // ── NEW: Periorbital / Crow's feet (8) ──
      130, 247, 30, 29, 359, 467, 260, 259,
      // ── NEW: Nasolabial fold (4) ──
      49, 220, 279, 440,
      // ── NEW: Corrugator supercilii (4) ──
      46, 53, 276, 283,
      // ── NEW: Mentalis (3) ──
      199, 175, 18,
      // ── NEW: Zygomatic + Temporalis (4) ──
      123, 352, 116, 345,
      // ── NEW: Forehead + Orbicularis Oris (5) ──
      151, 9, 87, 317, 164,
    ]),

    /* ── FACS Action Unit Definitions ── */
    AU_NAMES: Object.freeze({
      au1:  'Inner Brow Raise',        au2:  'Outer Brow Raise',
      au4:  'Brow Lowerer',            au5:  'Upper Lid Raise',
      au6:  'Cheek Raise',             au7:  'Lid Tightener',
      au9:  'Nose Wrinkler',           au10: 'Upper Lip Raiser',
      au12: 'Lip Corner Puller',       au14: 'Dimpler',
      au15: 'Lip Corner Depressor',    au17: 'Chin Raiser',
      au20: 'Lip Stretcher',           au23: 'Lip Tightener',
      au24: 'Lip Presser',             au25: 'Lips Part',
      au26: 'Jaw Drop',               au43: 'Eyes Closed',
      au45: 'Blink',                   au46: 'Wink',
    }),

    AU_LIST: Object.freeze([
      'au1','au2','au4','au5','au6','au7','au9','au10',
      'au12','au14','au15','au17','au20','au23','au24','au25','au26',
      'au43','au45','au46',
    ]),

    /* ── Expression Definitions ── */
    EXPRESSION_LABELS: Object.freeze({
      genuine_smile:   'Genuine Smile (Duchenne)',
      social_smile:    'Social Smile',
      suppressed_smile:'Suppressed Smile',
      surprise:        'Surprise',
      fear:            'Fear',
      anger:           'Anger',
      disgust:         'Disgust',
      contempt:        'Contempt',
      sadness:         'Sadness',
      anxiety:         'Anxiety',
      tension:         'Tension',
      concentration:   'Concentration',
      confusion:       'Confusion',
      disdain:         'Disdain',
      embarrassment:   'Embarrassment',
      pain:            'Pain',
      fatigue:         'Fatigue',
      boredom:         'Boredom',
      interest:        'Interest / Curiosity',
      pleasure:        'Pleasure',
      suppression:     'Emotion Suppression',
      au12_au14_asymmetry_pattern: 'AU12/AU14 Asymmetry Pattern',
      masked_sadness:  'Masked Sadness',
      nervous_smile:   'Nervous Smile',
      irritation:      'Irritation',
      melancholy:      'Melancholy',
      unease:          'Unease',
      distaste:        'Distaste',
      neutral:         'Neutral',
    }),

    /* ── Psychology Dimension Labels (12 dimensions) ── */
    PSYCHOLOGY_LABELS: Object.freeze({
      valence:            'Valence',
      arousal:            'Arousal',
      cognitiveLoad:      'Cognitive Load',
      stress:             'Stress',
      confidence:         'Confidence',
      engagement:         'Engagement',
      comfort:            'Comfort',
      authenticity:       'Expression Consistency',
      tension:            'Tension',
      patternAnomaly:     'Pattern Anomaly',
      suppressionEffort:  'Suppression Effort',
      emotionalRegulation:'Emotional Regulation',
    }),

    PSYCHOLOGY_KEYS: Object.freeze([
      'valence','arousal','cognitiveLoad','stress','confidence',
      'engagement','comfort','authenticity','tension','patternAnomaly',
      'suppressionEffort','emotionalRegulation',
    ]),

    /* ── Observable movement summaries (v3.11) ── */
    OBSERVABLE_LABELS: Object.freeze({
      browRaise: 'Brow Raise',
      browLower: 'Brow Lower',
      eyeOpening: 'Eye Opening',
      eyeClosure: 'Eye Closure',
      cheekRaise: 'Cheek Raise',
      noseWrinkle: 'Nose Wrinkle',
      lipCornerPull: 'Lip Corner Pull',
      lipCornerDepress: 'Lip Corner Depress',
      mouthOpening: 'Mouth Opening',
      lipCompression: 'Lip Compression',
      bilateralAsymmetry: 'Bilateral Asymmetry',
      motionEnergy: 'Motion Energy',
    }),

    OBSERVABLE_KEYS: Object.freeze([
      'browRaise','browLower','eyeOpening','eyeClosure','cheekRaise','noseWrinkle',
      'lipCornerPull','lipCornerDepress','mouthOpening','lipCompression',
      'bilateralAsymmetry','motionEnergy',
    ]),

    /* ── Temporal Dynamics Config ──
     *  Based on Hess & Kleck (1990), Cohn & Schmidt (2004)
     */
    TEMPORAL: Object.freeze({
      // Genuine smile onset: 0.5–0.7s (Ekman, 2003)
      genuineSmileOnsetMin: 350,   // ms
      genuineSmileOnsetMax: 900,   // ms
      // Fake smile onset: <200ms (too fast) or >1200ms (too slow)
      fakeSmileOnsetFast:   200,   // ms
      fakeSmileOnsetSlow:   1200,  // ms
      // Genuine expression duration: 0.5–4.0s
      genuineDurationMin:   500,   // ms
      genuineDurationMax:   4000,  // ms
      // AU velocity thresholds
      onsetVelocityMin:     0.003, // AU units/frame for onset detection
      offsetVelocityMin:    0.003,
      // Dynamics history length
      dynamicsHistoryLen:   10,    // frames
    }),

    /* ── Authenticity Scoring Weights ──
     *  Multi-factor scoring based on Ekman + Porter & ten Brinke (2008)
     */
    AUTHENTICITY: Object.freeze({
      duchenneWeight:       0.25,  // orbicularis oculi involvement
      temporalWeight:       0.20,  // onset/offset naturalness
      symmetryWeight:       0.15,  // bilateral symmetry
      durationWeight:       0.10,  // duration appropriateness
      muscleConflictWeight: 0.15,  // antagonist muscle activation
      leakageWeight:        0.15,  // micro-expression leakage
      // Asymmetry thresholds (Ekman et al., 1981)
      asymmetryGenuineMax:  0.15,  // genuine expressions: <15% asymmetry
      asymmetryFakedMin:    0.20,  // posed expressions: >20% asymmetry
    }),

    /* ── Transition Analysis Config ── */
    TRANSITION: Object.freeze({
      // Transition speed thresholds (milliseconds, FPS-independent)
      abruptMaxMs:       100,   // ≤100ms = abrupt  (~3 frames @30fps)
      smoothMinMs:       267,   // ≥267ms = smooth  (~8 frames @30fps)
      // Transition log size
      maxTransitions:    100,
      // Emotional inertia window (count of recent transitions)
      inertiaWindow:     30,
    }),

    /* ── Timing & Smoothing ── */
    PSYCHOLOGY: Object.freeze({
      historyLength:        300,   // ~10 sec @ 30fps (increased for deeper analysis)
      microExpressionMaxMs: 500,
      microExpressionMinMs: 40,
      baselineFrames:       90,
      smoothingAlpha:       0.10,  // slightly slower for stability
      blinkCooldownMs:      300,
      maxMicroLog:          100,   // increased
    }),

    /* ── Region Colors (for 78-point mesh - CYBER THEME) ── */
    REGION_COLORS: Object.freeze({
      brow:        '#00f0ff', // cy-cyan
      eye:         '#00f0ff',
      nose:        '#58a6ff', // cy-blue
      mouth:       '#b054ff', // cy-purple
      contour:     '#b054ff',
      cheek:       '#58a6ff',
      anchor:      '#8c8c8c',
      periorbital: '#b054ff',
      nasolabial:  '#58a6ff',
      corrugator:  '#00f0ff',
      mentalis:    '#b054ff',
      zygomatic:   '#58a6ff',
      forehead:    '#00f0ff',
    }),
  });

  global.FaceConfig = FACE_CONFIG;
})(typeof window !== 'undefined' ? window : globalThis);


```

## Source file: face-export.js

{
  "path": "face-export.js",
  "blob_sha": "8b853dcd99495b1573c0c5671850bd03b835a470",
  "sha256": "30f0871462ad80623ed5a3ddc79f5eb3cda41b854daaa43533055e9f7f7f739b",
  "lines": 169,
  "bytes": 6663,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 5,
    "face": 8
  },
  "url": "https://github.com/son-gladyso/FaceExpressionVFX/blob/29f7d4bfb2d687ac80541efd39537e61ad5b6e9c/face-export.js",
  "function_signature_candidates": []
}

```text

/* FaceVFX v3.11 observable-only JSON/CSV export. */
(function (global) {
  'use strict';

  function fmtTime(date) {
    var d = date || new Date();
    var pad = function (n) { return n < 10 ? '0' + n : String(n); };
    return d.getFullYear() + pad(d.getMonth() + 1) + pad(d.getDate()) + '_' +
      pad(d.getHours()) + pad(d.getMinutes()) + pad(d.getSeconds());
  }

  function triggerDownload(content, filename, mime) {
    var blob = new Blob([content], { type: mime });
    var url = URL.createObjectURL(blob);
    var anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = filename;
    document.body.appendChild(anchor);
    anchor.click();
    document.body.removeChild(anchor);
    setTimeout(function () { URL.revokeObjectURL(url); }, 5000);
  }

  function flattenResult(result) {
    if (!result) return {};
    var flat = {
      timestamp: new Date().toISOString(),
      schemaVersion: result.schemaVersion || '3.11.0',
      paused: result.paused ? 1 : 0,
      quality_score: result.quality ? result.quality.score.toFixed(4) : '',
      quality_action: result.quality ? result.quality.action : '',
      quality_flags: result.quality ? result.quality.reasons.join('|') : '',
      temporal_state: result.temporal ? result.temporal.label : '',
      temporal_confidence: result.temporal ? result.temporal.confidence.toFixed(4) : '',
      temporal_uncertainty: result.temporal ? result.temporal.entropy.toFixed(4) : '',
      inference_source: result.inferenceSource || 'rules',
      calibrated: result.calibrated ? 1 : 0,
    };
    var CFG = global.FaceConfig;
    if (CFG && result.auEstimates) {
      for (var i = 0; i < CFG.AU_LIST.length; i++) {
        var key = CFG.AU_LIST[i];
        var estimate = result.auEstimates[key];
        if (!estimate) continue;
        flat[key + '_value'] = estimate.value.toFixed(4);
        flat[key + '_lower'] = estimate.lower.toFixed(4);
        flat[key + '_upper'] = estimate.upper.toFixed(4);
        flat[key + '_confidence'] = estimate.confidence.toFixed(4);
      }
    }
    if (CFG && result.observables) {
      for (var o = 0; o < CFG.OBSERVABLE_KEYS.length; o++) {
        var observable = CFG.OBSERVABLE_KEYS[o];
        var interval = result.observables.intervals[observable];
        if (!interval) continue;
        flat['observable_' + observable] = interval.value.toFixed(4);
        flat['observable_' + observable + '_lower'] = interval.lower.toFixed(4);
        flat['observable_' + observable + '_upper'] = interval.upper.toFixed(4);
        flat['observable_' + observable + '_confidence'] = interval.confidence.toFixed(4);
      }
    }
    if (result.headPose) {
      flat.headPose_yaw = result.headPose.yaw.toFixed(2);
      flat.headPose_pitch = result.headPose.pitch.toFixed(2);
      flat.headPose_roll = result.headPose.roll.toFixed(2);
    }
    flat.blinkCount = result.blinkCount || 0;
    return flat;
  }

  function buildCSV(rows) {
    if (!rows || rows.length === 0) return '';
    var keys = Object.keys(rows[0]);
    var lines = [keys.join(',')];
    for (var i = 0; i < rows.length; i++) {
      var values = [];
      for (var j = 0; j < keys.length; j++) {
        var value = rows[i][keys[j]];
        var text = value == null ? '' : String(value);
        if (/[",\n]/.test(text)) text = '"' + text.replace(/"/g, '""') + '"';
        values.push(text);
      }
      lines.push(values.join(','));
    }
    return lines.join('\n');
  }

  function buildSnapshot(result) {
    if (!result) return null;
    return {
      timestamp: new Date().toISOString(),
      schemaVersion: result.schemaVersion || '3.11.0',
      notice: 'Observable facial geometry estimates only; not identity, intent, truthfulness, or diagnosis.',
      paused: Boolean(result.paused),
      quality: result.quality || null,
      calibration: result.calibration || null,
      auEstimates: result.auEstimates || {},
      observables: result.observables || null,
      temporal: result.temporal || null,
      headPose: result.headPose || null,
      blinkCount: result.blinkCount || 0,
      inference: {
        source: result.inferenceSource || 'rules',
        modelWeight: result.modelWeight || 0,
        modelMetadata: result.modelMetadata || null,
      },
    };
  }

  function flattenLightSnapshot(snapshot) {
    if (!snapshot) return {};
    var flat = {
      timestamp: snapshot.t ? new Date(snapshot.t).toISOString() : '',
      quality_score: snapshot.q == null ? '' : Number(snapshot.q).toFixed(4),
      quality_action: snapshot.qa || '',
      temporal_state: snapshot.ts || '',
      temporal_uncertainty: snapshot.tu == null ? '' : Number(snapshot.tu).toFixed(4),
    };
    var CFG = global.FaceConfig;
    if (CFG && snapshot.obs) {
      for (var i = 0; i < CFG.OBSERVABLE_KEYS.length; i++) {
        var key = CFG.OBSERVABLE_KEYS[i];
        flat['observable_' + key] = Number(snapshot.obs[key] || 0).toFixed(4);
      }
    }
    if (snapshot.hp) {
      flat.headPose_yaw = Number(snapshot.hp.y || 0).toFixed(2);
      flat.headPose_pitch = Number(snapshot.hp.p || 0).toFixed(2);
      flat.headPose_roll = Number(snapshot.hp.r || 0).toFixed(2);
    }
    return flat;
  }

  function downloadJSON(result) {
    triggerDownload(JSON.stringify(buildSnapshot(result), null, 2),
      'face-observables_' + fmtTime() + '.json', 'application/json');
  }
  function downloadCSV(result) {
    triggerDownload(buildCSV([flattenResult(result)]),
      'face-observables_' + fmtTime() + '.csv', 'text/csv');
  }
  function downloadHistoryJSON(sessions, profileName) {
    triggerDownload(JSON.stringify({
      exportTime: new Date().toISOString(), profile: profileName || 'unknown', sessions: sessions || [],
    }, null, 2), 'face-history_' + fmtTime() + '.json', 'application/json');
  }
  function downloadHistoryCSV(sessions) {
    var rows = [];
    for (var i = 0; i < (sessions || []).length; i++) {
      for (var j = 0; j < (sessions[i].snapshots || []).length; j++) {
        var row = flattenLightSnapshot(sessions[i].snapshots[j]);
        row.sessionId = sessions[i].id || i;
        rows.push(row);
      }
    }
    if (rows.length) triggerDownload(buildCSV(rows), 'face-history_' + fmtTime() + '.csv', 'text/csv');
  }

  global.FaceExport = Object.freeze({
    buildSnapshot: buildSnapshot,
    flattenResult: flattenResult,
    flattenLightSnapshot: flattenLightSnapshot,
    buildCSV: buildCSV,
    downloadJSON: downloadJSON,
    downloadCSV: downloadCSV,
    downloadHistoryJSON: downloadHistoryJSON,
    downloadHistoryCSV: downloadHistoryCSV,
  });
})(typeof window !== 'undefined' ? window : globalThis);


```

## Source file: face-helpers.js

{
  "path": "face-helpers.js",
  "blob_sha": "7de5c331e67f10cdabbca2965bbce24b846dee18",
  "sha256": "ec5e80c24b490467225d42d30e2de9a4d65734f15fa4d9abd37c8fe42b612910",
  "lines": 420,
  "bytes": 18374,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 26,
    "mouth": 4,
    "wrinkle": 2,
    "rig": 6,
    "face": 11
  },
  "url": "https://github.com/son-gladyso/FaceExpressionVFX/blob/29f7d4bfb2d687ac80541efd39537e61ad5b6e9c/face-helpers.js",
  "function_signature_candidates": []
}

```text

/* ═══════════════════════════════════════════════════════════════
 *  face-helpers.js  –  Math, Validation & Signal Processing
 *  v3 – Added: variance, slope, head pose, signal analysis
 * ═══════════════════════════════════════════════════════════════ */
(function (global) {
  'use strict';

  function dist(a, b) { return Math.hypot(a.x - b.x, a.y - b.y); }

  function dist3d(a, b) {
    return Math.hypot(a.x - b.x, a.y - b.y, (a.z || 0) - (b.z || 0));
  }

  function midpoint(a, b) {
    return { x: (a.x + b.x) * 0.5, y: (a.y + b.y) * 0.5 };
  }

  function angle(a, b, c) {
    var abx = b.x - a.x, aby = b.y - a.y;
    var acx = c.x - a.x, acy = c.y - a.y;
    return Math.atan2(abx * acy - aby * acx, abx * acx + aby * acy);
  }

  function clamp(value, min, max) {
    return value < min ? min : value > max ? max : value;
  }

  function lerp(a, b, t) { return a + (b - a) * t; }

  function ema(prev, curr, alpha) { return prev + alpha * (curr - prev); }

  function isValidFaceLandmarks(lm) {
    if (!Array.isArray(lm) || lm.length < 468) return false;
    var checks = [1, 33, 133, 152, 234, 263, 291, 362, 454];
    for (var i = 0; i < checks.length; i++) {
      var pt = lm[checks[i]];
      if (!pt || !Number.isFinite(pt.x) || !Number.isFinite(pt.y)) return false;
    }
    return true;
  }

  function avgY(lm, indices) {
    var sum = 0;
    for (var i = 0; i < indices.length; i++) sum += lm[indices[i]].y;
    return sum / indices.length;
  }

  function avgX(lm, indices) {
    var sum = 0;
    for (var i = 0; i < indices.length; i++) sum += lm[indices[i]].x;
    return sum / indices.length;
  }

  /* ── NEW: Variance of an array ── */
  function variance(arr) {
    if (!arr || arr.length < 2) return 0;
    var sum = 0, sumSq = 0, n = arr.length;
    for (var i = 0; i < n; i++) { sum += arr[i]; sumSq += arr[i] * arr[i]; }
    var mean = sum / n;
    return sumSq / n - mean * mean;
  }

  /* ── NEW: Linear regression slope (y vs index) ── */
  function slope(arr) {
    if (!arr || arr.length < 2) return 0;
    var n = arr.length;
    var sumX = 0, sumY = 0, sumXY = 0, sumXX = 0;
    for (var i = 0; i < n; i++) {
      sumX += i; sumY += arr[i];
      sumXY += i * arr[i]; sumXX += i * i;
    }
    var denom = n * sumXX - sumX * sumX;
    return denom === 0 ? 0 : (n * sumXY - sumX * sumY) / denom;
  }


  /* ── NEW: Periorbital wrinkle density (Crow's feet) ──
   *  Estimates orbicularis oculi contraction from landmark clustering.
   *  Higher density → more contraction → genuine smile indicator.
   */
  function periorbitalDensity(lm, periorbitalIdx, eyeOuterIdx) {
    if (!periorbitalIdx || periorbitalIdx.length < 2) return 0;
    var eyeOuter = lm[eyeOuterIdx];
    var totalDist = 0;
    for (var i = 0; i < periorbitalIdx.length; i++) {
      totalDist += dist(lm[periorbitalIdx[i]], eyeOuter);
    }
    var avgDist = totalDist / periorbitalIdx.length;
    // Smaller average distance = more contraction = higher density
    return Math.max(0, 1 - avgDist * 15);
  }

  /* ═══════════════════════════════════════════════════════════
   *  1€ FILTER (Casiez et al., CHI 2012)
   *  Speed-adaptive jitter reduction: high smoothing at rest,
   *  low latency during fast movement.
   *  State-of-the-art for real-time landmark filtering.
   * ═══════════════════════════════════════════════════════════ */
  function createOneEuroFilter(minCutoff, beta, dCutoff) {
    minCutoff = minCutoff || 1.0;
    beta      = beta || 0.007;
    dCutoff   = dCutoff || 1.0;
    var xPrev = null, dxPrev = 0, tPrev = null;

    function smoothingFactor(te, cutoff) {
      var r = 2 * Math.PI * cutoff * te;
      return r / (r + 1);
    }

    return {
      filter: function (x, timestamp) {
        if (tPrev === null) {
          tPrev = timestamp; xPrev = x; dxPrev = 0;
          return x;
        }
        var te = timestamp - tPrev;
        if (te <= 0) te = 1 / 30; // fallback ~30fps
        tPrev = timestamp;

        // Derivative estimation (low-pass filtered)
        var aD = smoothingFactor(te, dCutoff);
        var dx = (x - xPrev) / te;
        var dxHat = aD * dx + (1 - aD) * dxPrev;
        dxPrev = dxHat;

        // Adaptive cutoff based on speed
        var cutoff = minCutoff + beta * Math.abs(dxHat);
        var a = smoothingFactor(te, cutoff);
        var xHat = a * x + (1 - a) * xPrev;
        xPrev = xHat;
        return xHat;
      },
      reset: function () { xPrev = null; tPrev = null; dxPrev = 0; },
    };
  }

  /* ═══════════════════════════════════════════════════════════
   *  SAVITZKY-GOLAY SMOOTH DIFFERENTIATOR
   *  Computes smoothed 1st/2nd derivatives from ring buffer.
   *  Polynomial order 2, window 5 (symmetric).
   *  Based on Savitzky & Golay (1964), Analytical Chemistry.
   * ═══════════════════════════════════════════════════════════ */
  var SG_COEFF_D1 = Object.freeze([-2, -1, 0, 1, 2]); // 1st deriv, window 5
  var SG_NORM_D1  = 10;
  var SG_COEFF_D2 = Object.freeze([2, -1, -2, -1, 2]); // 2nd deriv, window 5
  var SG_NORM_D2  = 7;

  function sgDerivative(ringBuf, coeffs, norm) {
    // ringBuf must have >= 5 elements; uses last 5
    var n = ringBuf.length;
    if (n < 5) return 0;
    var sum = 0;
    for (var i = 0; i < 5; i++) {
      sum += coeffs[i] * ringBuf[n - 5 + i];
    }
    return sum / norm;
  }

  function sgVelocity(ringBuf) { return sgDerivative(ringBuf, SG_COEFF_D1, SG_NORM_D1); }
  function sgAcceleration(ringBuf) { return sgDerivative(ringBuf, SG_COEFF_D2, SG_NORM_D2); }

  /* ═══════════════════════════════════════════════════════════
   *  HEAD POSE ESTIMATION v2 — solvePnP-like
   *  Uses 6 canonical 3D model points (generic face model, mm)
   *  + perspective projection equations to estimate
   *  pitch/yaw/roll. Pure JS, no OpenCV dependency.
   *
   *  3D model points (origin at nose tip):
   *    Nose tip:       (0, 0, 0)
   *    Chin:           (0, -63.6, -12.5)
   *    Left eye outer: (-42.0, 32.7, -26.0)
   *    Right eye outer:(42.0, 32.7, -26.0)
   *    Left mouth:     (-28.0, -28.0, -24.1)
   *    Right mouth:    (28.0, -28.0, -24.1)
   *
   *  Reference: Kazemi & Sullivan (2014), "One millisecond
   *  face alignment with an ensemble of regression trees"
   * ═══════════════════════════════════════════════════════════ */
  var _MODEL_3D = [
    [0, 0, 0],              // nose tip
    [0, -63.6, -12.5],      // chin
    [-42.0, 32.7, -26.0],   // left eye outer
    [42.0, 32.7, -26.0],    // right eye outer
    [-28.0, -28.0, -24.1],  // left mouth
    [28.0, -28.0, -24.1],   // right mouth
  ];

  function computeHeadPoseV2(lm, _imgW, _imgH) {
    // imgW/imgH reserved for future full PnP solve
    var nose  = lm[1],  chin = lm[152];
    var leye  = lm[263], reye = lm[33];

    // Note: 2D-to-3D PnP is deferred; using robust geometric ratios
    // with z-depth for pose estimation (avoids OpenCV dependency).

    // Estimate yaw from left/right eye-to-nose distance ratio (robust)
    var dL = dist(leye, nose);
    var dR = dist(reye, nose);
    var yaw = Math.atan2(dL - dR, dL + dR + 1e-6) * 100; // degrees approx

    // Estimate pitch from nose-to-eyeMid vs nose-to-chin ratio using z-depth
    var eyeMidY = (leye.y + reye.y) * 0.5;
    var eyeMidZ = ((leye.z || 0) + (reye.z || 0)) * 0.5;
    var noseZ = nose.z || 0;
    var chinZ = chin.z || 0;
    var faceH3d = dist3d(
      { x: (leye.x + reye.x) * 0.5, y: eyeMidY, z: eyeMidZ },
      { x: chin.x, y: chin.y, z: chinZ }
    );
    var noseFromEye3d = dist3d(
      { x: nose.x, y: nose.y, z: noseZ },
      { x: (leye.x + reye.x) * 0.5, y: eyeMidY, z: eyeMidZ }
    );
    var pitchRatio = faceH3d > 0.01 ? noseFromEye3d / faceH3d : 0.33;
    var pitch = (pitchRatio - 0.33) * 100; // degrees approx

    // Roll from eye line angle (using z for tilt correction)
    var eyeDx = leye.x - reye.x;
    var eyeDy = leye.y - reye.y;
    var eyeDz = (leye.z || 0) - (reye.z || 0);
    var roll = Math.atan2(eyeDy, Math.sqrt(eyeDx * eyeDx + eyeDz * eyeDz)) * (180 / Math.PI);

    return { pitch: pitch, yaw: yaw, roll: roll };
  }

  /* ═══════════════════════════════════════════════════════════
   *  Z-DEPTH CURVATURE ANALYSIS
   *  Computes local curvature changes using z coordinates
   *  from MediaPipe Face Mesh. Higher curvature delta
   *  indicates more wrinkle/fold activity.
   * ═══════════════════════════════════════════════════════════ */
  function zCurvature(lm, indices) {
    if (!indices || indices.length < 3) return 0;
    var curvSum = 0, count = 0;
    for (var i = 1; i < indices.length - 1; i++) {
      var z0 = lm[indices[i - 1]].z || 0;
      var z1 = lm[indices[i]].z || 0;
      var z2 = lm[indices[i + 1]].z || 0;
      // Discrete second derivative of z (curvature proxy)
      curvSum += Math.abs(z0 - 2 * z1 + z2);
      count++;
    }
    return count > 0 ? curvSum / count : 0;
  }

  /* ═══════════════════════════════════════════════════════════
   *  LANDMARK CONFIDENCE WEIGHTING
   *  Uses MediaPipe's visibility field (0-1) to weight
   *  measurements. Low-visibility landmarks get reduced
   *  influence.
   * ═══════════════════════════════════════════════════════════ */
  function landmarkConfidence(lm, idx) {
    var pt = lm[idx];
    if (!pt) return 0;
    // MediaPipe provides 'visibility' for some landmarks
    if (typeof pt.visibility === 'number') return clamp(pt.visibility, 0, 1);
    // Fallback: use presence of z as indicator
    return (typeof pt.z === 'number' && Number.isFinite(pt.z)) ? 0.9 : 0.5;
  }

  function confidenceWeightedDist(lm, idxA, idxB) {
    var confA = landmarkConfidence(lm, idxA);
    var confB = landmarkConfidence(lm, idxB);
    var weight = Math.min(confA, confB);
    return { dist: dist(lm[idxA], lm[idxB]), weight: weight };
  }

  /* ═══════════════════════════════════════════════════════════
   *  BLINK RATE ANALYZER
   *  Scientific blink rate analysis (Stern et al., 1984).
   *  Normal: 15-20 blinks/min.
   *  Elevated: >25 → stress/cognitive load indicator.
   *  Reduced: <10 → concentration/fatigue indicator.
   *  Also computes inter-blink interval (IBI) variance.
   * ═══════════════════════════════════════════════════════════ */
  function createBlinkAnalyzer() {
    var blinkTimestamps = [];   // last N blink times
    var maxHistory = 60;        // keep last 60 blinks
    var ibiHistory = [];        // inter-blink intervals (ms)
    var clusterCount = 0;       // completed rapid blink clusters (≥3 rapid blinks)
    var currentRun = 0;         // consecutive rapid blinks in current sequence
    var lastBlinkCount = 0;     // internal blink-count tracker (encapsulated)
    // PERCLOS tracking (Dinges et al., 1998)
    // Percentage of time eyes are ≥80% closed over a rolling window.
    // PERCLOS > 0.15 (15%) is the gold-standard drowsiness indicator.
    // O(1) circular buffer: avoids O(n) shift on 900-element arrays.
    var PERCLOS_CAP = 900;      // 30 sec @ 30fps
    var perclosBuf = new Uint8Array(PERCLOS_CAP);
    var perclosIdx = 0;
    var perclosLen = 0;
    var perclosClosedCount = 0; // running tally of closed frames

    return {
      /** Record a blink only if the running count has increased.
       *  Encapsulates the _lastCount tracking that was previously
       *  done externally in face-detector.js. */
      recordBlinkIfNew: function (currentCount, timestamp) {
        if (currentCount > lastBlinkCount) {
          this.recordBlink(timestamp);
        }
        lastBlinkCount = currentCount;
      },

      recordBlink: function (timestamp) {
        if (blinkTimestamps.length > 0) {
          var lastBlink = blinkTimestamps[blinkTimestamps.length - 1];
          var ibi = timestamp - lastBlink;
          ibiHistory.push(ibi);
          if (ibiHistory.length > maxHistory) ibiHistory.shift();
          // Detect blink cluster: ≥3 consecutive blinks with IBI < 400ms
          // A cluster is counted once when the 3rd rapid blink completes.
          // Subsequent rapid blinks in the same burst don't count again.
          if (ibi < 400) {
            currentRun++;
            if (currentRun === 2) clusterCount++; // 3rd blink = 2nd short IBI → cluster
          } else {
            currentRun = 0;
          }
        }
        blinkTimestamps.push(timestamp);
        if (blinkTimestamps.length > maxHistory) blinkTimestamps.shift();
      },

      getBlinkRate: function () {
        // Blinks per minute over last 60 seconds
        if (blinkTimestamps.length < 2) return 0;
        var now = blinkTimestamps[blinkTimestamps.length - 1];
        var windowMs = 60000; // 1 minute
        var count = 0;
        for (var i = blinkTimestamps.length - 1; i >= 0; i--) {
          if (now - blinkTimestamps[i] > windowMs) break;
          count++;
        }
        var elapsed = Math.min(windowMs, now - blinkTimestamps[0]);
        return elapsed > 0 ? (count / elapsed) * 60000 : 0;
      },

      getIBIVariance: function () {
        return variance(ibiHistory);
      },

      getIBIMean: function () {
        if (ibiHistory.length === 0) return 0;
        var sum = 0;
        for (var i = 0; i < ibiHistory.length; i++) sum += ibiHistory[i];
        return sum / ibiHistory.length;
      },

      /** IBI Coefficient of Variation (CV = std / mean).
       *  Dimensionless measure of blink regularity, more robust
       *  than raw variance (which is in ms² and scale-dependent).
       *  CV > 0.5 indicates irregular blink timing.
       *  Reference: Reed & Green (1999) – heart/blink rate variability. */
      getIBICoeffVar: function () {
        if (ibiHistory.length < 2) return 0;
        var mean = this.getIBIMean();
        if (mean < 1) return 0;
        return Math.sqrt(variance(ibiHistory)) / mean;
      },

      getClusterCount: function () { return clusterCount; },

      /** PERCLOS: record whether eyes are closed this frame.
       *  Call once per frame with the current EAR value.
       *  O(1) circular buffer with running count.
       *  Reference: Dinges et al. (1998) – PERCLOS drowsiness metric. */
      recordEyeState: function (earAvg) {
        var isClosed = earAvg < 0.16 ? 1 : 0;
        if (perclosLen === PERCLOS_CAP) {
          // Buffer full: subtract oldest entry before overwrite
          perclosClosedCount -= perclosBuf[perclosIdx];
        } else {
          perclosLen++;
        }
        perclosBuf[perclosIdx] = isClosed;
        perclosClosedCount += isClosed;
        perclosIdx = (perclosIdx + 1) % PERCLOS_CAP;
      },

      /** Returns PERCLOS value (0–1). >0.15 = drowsy. O(1). */
      getPERCLOS: function () {
        return perclosLen > 0 ? perclosClosedCount / perclosLen : 0;
      },

      // Interpretation (Stern et al., 1984)
      interpret: function () {
        var rate = this.getBlinkRate();
        var ibiCV = this.getIBICoeffVar();
        var clusters = this.getClusterCount();
        var perclos = this.getPERCLOS();
        var base = { rate: rate, clusters: clusters, ibiVariance: ibiCV, perclos: perclos };
        var _t = function (k, v) { var s = (typeof FaceI18n !== 'undefined') ? FaceI18n.t(k) : k; return s.replace('{0}', v); };
        // PERCLOS > 0.15 is gold-standard drowsiness (Dinges et al., 1998)
        if (perclos > 0.15) return Object.assign(base, { level: 'drowsy', stressIndicator: 0.2, note: _t('blink.drowsy', (perclos*100).toFixed(0)) });
        if (rate > 25) return Object.assign(base, { level: 'elevated', stressIndicator: 0.7, note: _t('blink.elevated', rate.toFixed(0)) });
        if (rate < 10 && rate > 0) return Object.assign(base, { level: 'reduced', stressIndicator: 0.3, note: _t('blink.reduced', rate.toFixed(0)) });
        if (ibiCV > 0.5) return Object.assign(base, { level: 'irregular', stressIndicator: 0.5, note: _t('blink.irregular', ibiCV.toFixed(2)) });
        return Object.assign(base, { level: 'normal', stressIndicator: 0, note: '' });
      },
    };
  }

  global.FaceHelpers = Object.freeze({
    dist: dist, dist3d: dist3d, midpoint: midpoint, angle: angle,
    clamp: clamp, lerp: lerp, ema: ema,
    isValidFaceLandmarks: isValidFaceLandmarks,
    avgY: avgY, avgX: avgX,
    variance: variance, slope: slope,
    computeHeadPoseV2: computeHeadPoseV2,
    periorbitalDensity: periorbitalDensity,
    // NEW: State-of-the-art additions
    createOneEuroFilter: createOneEuroFilter,
    sgVelocity: sgVelocity,
    sgAcceleration: sgAcceleration,
    zCurvature: zCurvature,
    landmarkConfidence: landmarkConfidence,
    confidenceWeightedDist: confidenceWeightedDist,
    createBlinkAnalyzer: createBlinkAnalyzer,
  });
})(typeof window !== 'undefined' ? window : globalThis);


```
