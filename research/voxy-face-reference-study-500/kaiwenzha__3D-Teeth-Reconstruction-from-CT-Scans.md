# Facial rendering source scan: kaiwenzha/3D-Teeth-Reconstruction-from-CT-Scans

Repository: https://github.com/kaiwenzha/3D-Teeth-Reconstruction-from-CT-Scans

Pinned source revision: 124cb0163a63789cba7b5486b887790f217e15d7

Discovery topic: teeth 3D

Description: SJTU-CS337 Computer Graphics Course Project

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"teeth": ["RefineNet & SESNet/convert_teeth_to_tfrecords.py"]}

## Source file: RefineNet & SESNet/convert_teeth_to_tfrecords.py

{
  "path": "RefineNet & SESNet/convert_teeth_to_tfrecords.py",
  "blob_sha": "2d1905ba1c4f559ac656de05e68a7dc011a1fb85",
  "sha256": "d43b6d99be4ec6e4a09f8c18571b0976e1daedbd5ebe18cf8f4fbe975c9c926b",
  "lines": 24,
  "bytes": 892,
  "selection_score": 15,
  "keyword_line_counts": {},
  "url": "https://github.com/kaiwenzha/3D-Teeth-Reconstruction-from-CT-Scans/blob/124cb0163a63789cba7b5486b887790f217e15d7/RefineNet%20%26%20SESNet/convert_teeth_to_tfrecords.py",
  "function_signature_candidates": []
}

```text

import os
from utils.tf_records import write_image_annotation_pairs_to_tfrecord

image_root_path = './data/images'
label_root_path = './data/labels'

image_label_pairs = []
for mid_path in os.listdir(label_root_path):
	image_label_pairs.append([])
	for post_path in os.listdir(os.path.join(label_root_path, mid_path)):
		image_label_pairs[-1].append((os.path.join(image_root_path, mid_path, post_path), os.path.join(label_root_path, mid_path, post_path)))
	image_label_pairs[-1].sort()

train_pairs = image_label_pairs[0] + image_label_pairs[1]
test_pairs = image_label_pairs[1]

write_image_annotation_pairs_to_tfrecord(filename_pairs=train_pairs,
                                         tfrecords_filename='./data/train.tfrecords')

write_image_annotation_pairs_to_tfrecord(filename_pairs=test_pairs,
                                         tfrecords_filename='./data/test.tfrecords')





```

## Source file: SegNet/train.py

{
  "path": "SegNet/train.py",
  "blob_sha": "22d59db6953f179e7eb06d3262a241dd90e6e641",
  "sha256": "7b2ecd6e42aeca2417169531d66737c69fd37acf7b96a6097b86fd61b064cb4e",
  "lines": 109,
  "bytes": 3778,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/kaiwenzha/3D-Teeth-Reconstruction-from-CT-Scans/blob/124cb0163a63789cba7b5486b887790f217e15d7/SegNet/train.py",
  "function_signature_candidates": [
    {
      "line": 29,
      "text": "def main():"
    }
  ]
}

```text

import os

import tensorflow as tf
import tensorflow.contrib.slim as slim

import SegNetCMR


WORKING_DIR = os.getcwd()
TRAINING_DIR = os.path.join(WORKING_DIR, 'Data', 'Training')
TEST_DIR = os.path.join(WORKING_DIR, 'Data', 'Test')

ROOT_LOG_DIR = os.path.join(WORKING_DIR, 'Output')
RUN_NAME = "Run_double"
LOG_DIR = os.path.join(ROOT_LOG_DIR, RUN_NAME)
TRAIN_WRITER_DIR = os.path.join(LOG_DIR, 'Train')
TEST_WRITER_DIR = os.path.join(LOG_DIR, 'Test')

CHECKPOINT_FN = 'model.ckpt'
CHECKPOINT_FL = os.path.join(LOG_DIR, CHECKPOINT_FN)


BATCH_NORM_DECAY = 0.95 #Start off at 0.9, then increase.
MAX_STEPS = 10000
BATCH_SIZE = 3
SAVE_INTERVAL = 50

TEST = True
def main():
    training_data = SegNetCMR.GetData(TRAINING_DIR)
    test_data = SegNetCMR.GetData(TEST_DIR)

    g = tf.Graph()

    with g.as_default():

        images, labels, is_training = SegNetCMR.placeholder_inputs(batch_size=BATCH_SIZE)

        arg_scope = SegNetCMR.inference_scope(is_training=True, batch_norm_decay=BATCH_NORM_DECAY)

        with slim.arg_scope(arg_scope):
            logits = SegNetCMR.inference(images, class_inc_bg=2)

        SegNetCMR.add_output_images(images=images, logits=logits, labels=labels)

        loss = SegNetCMR.loss_calc(logits=logits, labels=labels)

        train_op, global_step = SegNetCMR.training(loss=loss, learning_rate=1e-04)

        accuracy = SegNetCMR.evaluation(logits=logits, labels=labels)

        summary = tf.summary.merge_all()

        init = tf.global_variables_initializer()

        saver = tf.train.Saver([x for x in tf.global_variables() if 'Adam' not in x.name])

        sm = tf.train.SessionManager()

        with sm.prepare_session("", init_op=init, saver=saver, checkpoint_dir=LOG_DIR) as sess:

            sess.run(tf.variables_initializer([x for x in tf.global_variables() if 'Adam' in x.name]))

            train_writer = tf.summary.FileWriter(TRAIN_WRITER_DIR, sess.graph)
            test_writer = tf.summary.FileWriter(TEST_WRITER_DIR)

            global_step_value, = sess.run([global_step])

            print("Last trained iteration was: ", global_step_value)

            for step in range(global_step_value+1, global_step_value+MAX_STEPS+1):

                print("Iteration: ", step)

                images_batch, labels_batch = training_data.next_batch(BATCH_SIZE)

                train_feed_dict = {images: images_batch,
                                   labels: labels_batch,
                                   is_training: True}

                _, train_loss_value, train_accuracy_value, train_summary_str = sess.run([train_op, loss, accuracy, summary], feed_dict=train_feed_dict)

                if step % SAVE_INTERVAL == 0 and TEST:

                    print("Train Loss: ", train_loss_value)
                    print("Train accuracy: ", train_accuracy_value)
                    train_writer.add_summary(train_summary_str, step)
                    train_writer.flush()

                    images_batch, labels_batch = test_data.next_batch(BATCH_SIZE)

                    test_feed_dict = {images: images_batch,
                                      labels: labels_batch,
                                      is_training: False}

                    test_loss_value, test_accuracy_value, test_summary_str = sess.run([loss, accuracy, summary], feed_dict=test_feed_dict)

                    print("Test Loss: ", test_loss_value)
                    print("Test accuracy: ", test_accuracy_value)
                    test_writer.add_summary(test_summary_str, step)
                    test_writer.flush()

                    saver.save(sess, CHECKPOINT_FL, global_step=step)
                    print("Session Saved")
                    print("================")


if __name__ == '__main__':
    main()

```

## Source file: SegNet/SegNetCMR/inputs.py

{
  "path": "SegNet/SegNetCMR/inputs.py",
  "blob_sha": "8e4c6cf3877395225a6ec33d4aab6cf831e6172a",
  "sha256": "ee9270867e89ca5bf05f97311de5648c1722d82f225de66b5eda10e21a23a86e",
  "lines": 9,
  "bytes": 272,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/kaiwenzha/3D-Teeth-Reconstruction-from-CT-Scans/blob/124cb0163a63789cba7b5486b887790f217e15d7/SegNet/SegNetCMR/inputs.py",
  "function_signature_candidates": [
    {
      "line": 3,
      "text": "def placeholder_inputs(batch_size):"
    }
  ]
}

```text

import tensorflow as tf

def placeholder_inputs(batch_size):

    images = tf.placeholder(tf.float32, [batch_size, 256, 256, 1])
    labels = tf.placeholder(tf.int64, [batch_size, 256, 256])
    is_training = tf.placeholder(tf.bool)

    return images, labels, is_training

```

## Source file: SegNet/SegNetCMR/layers.py

{
  "path": "SegNet/SegNetCMR/layers.py",
  "blob_sha": "56e616929ce2b5e8229be6edd36172228c0feaca",
  "sha256": "99e407d99cf2e12cc5e6b4078d9de6859d0e5cf183f188a2da25ea71ca913199",
  "lines": 49,
  "bytes": 2001,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/kaiwenzha/3D-Teeth-Reconstruction-from-CT-Scans/blob/124cb0163a63789cba7b5486b887790f217e15d7/SegNet/SegNetCMR/layers.py",
  "function_signature_candidates": [
    {
      "line": 9,
      "text": "def unpool_with_argmax(pool, ind, name = None, ksize=[1, 2, 2, 1]):"
    },
    {
      "line": 40,
      "text": "def _MaxPoolGradWithArgmax(op, grad, unused_argmax_grad):"
    }
  ]
}

```text

import numpy as np

import tensorflow as tf

from tensorflow.python.ops import gen_nn_ops
from tensorflow.python.framework import ops


def unpool_with_argmax(pool, ind, name = None, ksize=[1, 2, 2, 1]):

    """
       Unpooling layer after max_pool_with_argmax.
       Args:
           pool:   max pooled output tensor
           ind:      argmax indices
           ksize:     ksize is the same as for the pool
       Return:
           unpool:    unpooling tensor
    """
    with tf.variable_scope(name):
        input_shape = pool.get_shape().as_list()
        output_shape = (input_shape[0], input_shape[1] * ksize[1], input_shape[2] * ksize[2], input_shape[3])

        flat_input_size = np.prod(input_shape)
        flat_output_shape = [output_shape[0], output_shape[1] * output_shape[2] * output_shape[3]]

        pool_ = tf.reshape(pool, [flat_input_size])
        batch_range = tf.reshape(tf.range(output_shape[0], dtype=ind.dtype), shape=[input_shape[0], 1, 1, 1])
        b = tf.ones_like(ind) * batch_range
        b = tf.reshape(b, [flat_input_size, 1])
        ind_ = tf.reshape(ind, [flat_input_size, 1])
        ind_ = tf.concat([b, ind_], 1) #交换了两个参数--remove!!

        ret = tf.scatter_nd(ind_, pool_, shape=flat_output_shape)
        ret = tf.reshape(ret, output_shape)
        return ret

try:
    @ops.RegisterGradient("MaxPoolWithArgmax")
    def _MaxPoolGradWithArgmax(op, grad, unused_argmax_grad):
        return gen_nn_ops._max_pool_grad_with_argmax(op.inputs[0],
                                                     grad,
                                                     op.outputs[1],
                                                     op.get_attr("ksize"),
                                                     op.get_attr("strides"),
                                                     padding=op.get_attr("padding"))
except Exception as e:
    print(f"Could not add gradient for MaxPoolWithArgMax, Likely installed already (tf 1.4)")
    print(e)


```
