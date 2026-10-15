# Facial rendering source scan: rickie95/3DMMRegistration

Repository: https://github.com/rickie95/3DMMRegistration

Pinned source revision: 360f52de184fa063a61e05f184a6de6a42d4f670

Discovery topic: face model 3D

Description: Face model registration tool

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"face": ["graphicInterface/conf_param_window.py", "graphicInterface/console.py", "graphicInterface/file_dialogs.py", "graphicInterface/main_widget.py", "graphicInterface/plot_button_collection.py", "graphicInterface/plot_figure.py", "graphicInterface/plot_interactive_figure.py", "graphicInterface/rotatable_figure.py", "graphicInterface/show_displacement.py", "graphicInterface/upper_toolbar.py", "graphicInterface/upper_toolbar_controls.py", "graphicInterface/window.py"]}

## Source file: graphicInterface/window.py

{
  "path": "graphicInterface/window.py",
  "blob_sha": "90d9a44ec2660dfb82bd2eec69343d682c7d92b4",
  "sha256": "cc9980b9c42d362975745f71a51c14754c6361e00b5c026f350fc0b91f33f31a",
  "lines": 55,
  "bytes": 1616,
  "selection_score": 3,
  "keyword_line_counts": {
    "face": 3
  },
  "url": "https://github.com/rickie95/3DMMRegistration/blob/360f52de184fa063a61e05f184a6de6a42d4f670/graphicInterface/window.py",
  "function_signature_candidates": [
    {
      "line": 10,
      "text": "def __init__(self):"
    },
    {
      "line": 17,
      "text": "def initUI(self):"
    },
    {
      "line": 30,
      "text": "def center(self):"
    },
    {
      "line": 36,
      "text": "def setStatus(self, message):"
    },
    {
      "line": 39,
      "text": "def setStatusReady(self):"
    },
    {
      "line": 45,
      "text": "def __init__(self):"
    },
    {
      "line": 49,
      "text": "def mouseDoubleClickEvent(self, *args, **kwargs):"
    },
    {
      "line": 53,
      "text": "def update_parameter_label(self):"
    }
  ]
}

```text

from graphicInterface.main_widget import *
from graphicInterface.console import Logger
from pointRegistration.registration_param import RegistrationParameters
from PyQt5.QtGui import QIcon
from graphicInterface.conf_param_window import EditConfWindow


class Window(QMainWindow):

    def __init__(self):
        super().__init__()
        self.setWindowIcon(QIcon('resources/icon.png'))
        Logger()
        self.statusLabel = QLabel("")
        self.initUI()

    def initUI(self):
        self.mainWidget = MainWidget(self)
        self.setCentralWidget(self.mainWidget)
        self.resize(1000, 600)
        self.center()
        statusBar = QStatusBar()
        self.setStatusBar(statusBar)
        statusBar.addWidget(self.statusLabel)
        statusBar.addPermanentWidget(ConfigLabel())
        self.setWindowTitle('Shape Registrator')
        self.setStatus("Ready.")
        self.show()

    def center(self):
        qr = self.frameGeometry()
        cp = QDesktopWidget().availableGeometry().center()
        qr.moveCenter(cp)
        self.move(qr.topLeft())

    def setStatus(self, message):
        self.statusLabel.setText(message)

    def setStatusReady(self):
        self.setStatus("Ready")


class ConfigLabel(QLabel):

    def __init__(self):
        super(ConfigLabel, self).__init__()
        self.update_parameter_label()

    def mouseDoubleClickEvent(self, *args, **kwargs):
        window = EditConfWindow(self)
        window.show()

    def update_parameter_label(self):
        ss = "<b>(Double click to edit) </b>  |" + RegistrationParameters.to_string()
        self.setText(ss)


```

## Source file: graphicInterface/console.py

{
  "path": "graphicInterface/console.py",
  "blob_sha": "7e1cae61379b98c53ac9a665134e0977e744de65",
  "sha256": "be1d5381bcd766f2cd8feee518c87d1963a773eed5fdcb85f74746337c05ccd0",
  "lines": 80,
  "bytes": 2488,
  "selection_score": 3,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/rickie95/3DMMRegistration/blob/360f52de184fa063a61e05f184a6de6a42d4f670/graphicInterface/console.py",
  "function_signature_candidates": [
    {
      "line": 15,
      "text": "def __init__(self):"
    },
    {
      "line": 22,
      "text": "def save_onfile(self):"
    },
    {
      "line": 29,
      "text": "def __init__(self):"
    },
    {
      "line": 34,
      "text": "def addRow(row):"
    },
    {
      "line": 43,
      "text": "def setParent(parent):"
    },
    {
      "line": 48,
      "text": "def save_log():"
    },
    {
      "line": 63,
      "text": "def __init__(self):"
    },
    {
      "line": 69,
      "text": "def __enter__(self):"
    },
    {
      "line": 74,
      "text": "def __exit__(self, *_):"
    }
  ]
}

```text

from PyQt5.QtWidgets import QPlainTextEdit
from PyQt5.QtGui import QTextCursor
import datetime
import time
import os


class Logger:

    instance = None
    lock = False

    class _PrivateLog(QPlainTextEdit):

        def __init__(self):
            super().__init__()
            self.setReadOnly(True)
            self.setLineWrapMode(QPlainTextEdit.NoWrap)
            self.cursor = QTextCursor(self.document())
            self.setTextCursor(self.cursor)

        def save_onfile(self):
            now = datetime.datetime.now()
            filename = ("log-%d-%d_%d-%d-%d.txt" % (now.hour, now.minute, now.year, now.month, now.day))
            file = open(filename, "w")
            file.write(self.toPlainText())
            file.close()

    def __init__(self):
        if Logger.instance is None:
            Logger.instance = self._PrivateLog()

    @staticmethod
    def addRow(row):
        if Logger.instance is not None:
            Logger.instance.insertPlainText(row+"\n")
            time.sleep(0.1)
            Logger.instance.moveCursor(QTextCursor.End)
            time.sleep(0.1)
        print(row)

    @staticmethod
    def setParent(parent):
        if Logger.instance is not None:
            Logger.instance.setParent(parent)

    @staticmethod
    def save_log():
        if Logger.instance is not None:
            Logger.instance.save_onfile()


class suppress_stdout_stderr(object):
    """
        A context manager for doing a "deep suppression" of stdout and stderr in
        Python, i.e. will suppress all print, even if the print originates in a
        compiled C/Fortran sub-function.
        This will not suppress raised exceptions, since exceptions are printed
        to stderr just before a script exits, and after the context manager has
        exited.
    """

    def __init__(self):
        # Open a pair of null files
        self.null_fds = [os.open(os.devnull, os.O_RDWR) for x in range(2)]
        # Save the actual stdout (1) and stderr (2) file descriptors.
        self.save_fds = [os.dup(1), os.dup(2)]

    def __enter__(self):
        # Assign the null pointers to stdout and stderr.
        os.dup2(self.null_fds[0], 1)
        os.dup2(self.null_fds[1], 2)

    def __exit__(self, *_):
        # Re-assign the real stdout/stderr back to (1) and (2)
        os.dup2(self.save_fds[0], 1)
        os.dup2(self.save_fds[1], 2)
        # Close the null files
        for fd in self.null_fds + self.save_fds:
            os.close(fd)

```

## Source file: graphicInterface/main_widget.py

{
  "path": "graphicInterface/main_widget.py",
  "blob_sha": "d4bf6a8237e57bc380a1c7db55b58a69725cf5c6",
  "sha256": "615c1280e8dc3d244aed6811d224afd33bcfe245aa5bc510daa246c50718570f",
  "lines": 160,
  "bytes": 6721,
  "selection_score": 3,
  "keyword_line_counts": {
    "rig": 4,
    "face": 4
  },
  "url": "https://github.com/rickie95/3DMMRegistration/blob/360f52de184fa063a61e05f184a6de6a42d4f670/graphicInterface/main_widget.py",
  "function_signature_candidates": [
    {
      "line": 12,
      "text": "def __init__(self, parent):"
    },
    {
      "line": 25,
      "text": "def initUI(self):"
    },
    {
      "line": 39,
      "text": "def load_target(self):"
    },
    {
      "line": 52,
      "text": "def load_source(self):"
    },
    {
      "line": 64,
      "text": "def restore(self):"
    },
    {
      "line": 70,
      "text": "def restore_highlight(self):"
    },
    {
      "line": 73,
      "text": "def restore_target(self):"
    },
    {
      "line": 76,
      "text": "def landmark_selected(self, colors):"
    },
    {
      "line": 79,
      "text": "def data_selected(self, x_coord, y_coord, width, height):    # apply color to target"
    },
    {
      "line": 82,
      "text": "def registrate(self, method, percentage):"
    },
    {
      "line": 99,
      "text": "def stop_registration_thread(self):"
    },
    {
      "line": 104,
      "text": "def show_displacement_map(self):"
    },
    {
      "line": 108,
      "text": "def save_target(self):"
    },
    {
      "line": 119,
      "text": "def registration_completed_callback(self, model):"
    },
    {
      "line": 137,
      "text": "def registrate_batch_callback(self):"
    },
    {
      "line": 145,
      "text": "def registrate_batch(self, method, percentage, filenames):"
    },
    {
      "line": 159,
      "text": "def savelog_onfile():"
    }
  ]
}

```text

import os
from graphicInterface.plot_interactive_figure import PlotInteractiveFigure
from graphicInterface.rotatable_figure import RotatableFigure
from graphicInterface.show_displacement import DisplacementMapWindow
from graphicInterface.upper_toolbar import *
from pointRegistration.batchRegistration import BatchRegistrationThread
from pointRegistration.model import Model
from pointRegistration.registration import Registration


class MainWidget(QWidget):
    def __init__(self, parent):
        super().__init__(parent)
        Logger.addRow(str("Starting up.."))
        self.source_model = Model(os.path.join(".", "data", "avgModel_bh_1779_NE.mat"))
        self.target_model = None
        self.sx_widget = None
        self.dx_widget = None
        self.registration_thread = None
        self.toolbar = None
        self.registrated = None
        Logger.addRow(str("Ready."))
        self.initUI()

    def initUI(self):
        grid_central = QGridLayout(self)
        self.setLayout(grid_central)
        self.sx_widget = PlotInteractiveFigure(self, self.source_model, title="Source")
        self.dx_widget = RotatableFigure(self, None, title="Target")
        grid_central.addWidget(self.sx_widget, 1, 0, 1, 2)
        self.sx_widget.draw_data()
        grid_central.addWidget(self.dx_widget, 1, 2, 1, 2)
        self.dx_widget.draw_data()
        self.toolbar = UpperToolbar(self)
        grid_central.addWidget(self.toolbar, 0, 0, 1, 4)
        grid_central.setRowStretch(0, 1)
        grid_central.setRowStretch(1, 30)

    def load_target(self):
        filters = "OFF Files (*.off);;WRML Files (*.wrml);;MAT Files (*.mat)"
        file_name = load_file_dialog(self, filters)
        if file_name is None:
            return

        self.toolbar.start_registration_button.setEnabled(True)
        self.target_model = Model(file_name)
        self.dx_widget.load_model(self.target_model)
        self.dx_widget.draw_data(clear=True)
        Logger.addRow(str("File loaded correctly: " + file_name))
        self.toolbar.save_target_btn.setEnabled(True)

    def load_source(self):
        filters = "OFF Files (*.off);;WRML Files (*.wrml);;MAT Files (*.mat)"
        file_name = load_file_dialog(self, filters)
        if file_name is None:
            return

        self.toolbar.start_registration_button.setEnabled(True)
        self.source_model = Model(file_name)
        self.sx_widget.load_model(self.source_model)
        self.sx_widget.draw_data()
        Logger.addRow(str("File loaded correctly: " + file_name))

    def restore(self):
        self.restore_highlight()
        if self.dx_widget.has_model():
            self.restore_target()
            self.toolbar.start_registration_button.setEnabled(True)

    def restore_highlight(self):
        self.sx_widget.highlight_data([-1])

    def restore_target(self):
        self.dx_widget.restore_model()

    def landmark_selected(self, colors):
        self.dx_widget.set_landmarks_colors(colors)

    def data_selected(self, x_coord, y_coord, width, height):    # apply color to target
        self.dx_widget.select_area(x_coord, y_coord, width, height)

    def registrate(self, method, percentage):
        if not self.sx_widget.there_are_points_highlighted():  # names are everything
            QMessageBox.critical(self, 'Error', "No rigid points have been selected.")
            raise Exception("No rigid points selected")

        if self.dx_widget.model is None:
            QMessageBox.critical(self, 'Error', "Please, load a target model.")
            raise Exception("Target model is not present.")

        if self.registration_thread is None:
            self.toolbar.show_displacement_btn.setEnabled(False)
            self.parent().setStatus("Busy...")
            self.registration_thread = Registration(method, self.source_model, self.target_model, percentage,
                                                    self.registration_completed_callback,
                                                    self.dx_widget.update_plot_callback)
            self.registration_thread.start()

    def stop_registration_thread(self):
        if self.registration_thread is not None:
            Logger.addRow(str("Trying to stop registration thread..."))
            self.registration_thread.stop()

    def show_displacement_map(self):
        DisplacementMapWindow(self.parent(), self.source_model.compute_displacement_map(self.target_model, 3))  # FIXME
        self.parent().setStatusReady()

    def save_target(self):
        if self.target_model is None:
            QMessageBox.critical(self, 'Error', "The source model was not registered yet.")
            return

        filters = "MAT File (*.mat);;OFF File (*.off);;"
        filename = save_file_dialog(self, filters)
        if filename is None:
            return
        self.target_model.save_model(filename) #fixme controllare che venga realmente salvato

    def registration_completed_callback(self, model):
        Logger.addRow(str("Registration completed."))
        self.target_model.bgImage = self.dx_widget.bgImage
        self.dx_widget.clear()

        self.dx_widget.set_secondary_model(model)
        self.dx_widget.load_model(self.target_model)
        self.target_model = model

        self.dx_widget.draw_data()
        self.parent().setStatusReady()
        self.registration_thread = None
        self.toolbar.stop_registration_button.setEnabled(False)
        self.toolbar.show_displacement_btn.setEnabled(True)
        self.registrated = True
        self.parent().setStatus("Registration completed, displacement map available. Click Show Displacement Map.")
        # Target and Source are now plotted in target widget

    def registrate_batch_callback(self):
        try:
            Logger.addRow(str("Registration completed."))
            self.registration_thread = None
            self.parent().setStatus("Ready.")
        except Exception as ex:
            print(ex)

    def registrate_batch(self, method, percentage, filenames):

        if not self.sx_widget.there_are_points_highlighted():  # names are everything
            QMessageBox.critical(self, 'Error', "No rigid points have been selected.")
            raise Exception("No rigid points selected")

        if self.registration_thread is None and self.sx_widget.there_are_points_highlighted():
            self.parent().setStatus("Busy...")

            self.registration_thread = BatchRegistrationThread(self.sx_widget.model, filenames, percentage,
                                                               self.registrate_batch_callback)
            self.registration_thread.start()

    @staticmethod
    def savelog_onfile():
        Logger.save_log()


```

## Source file: graphicInterface/plot_figure.py

{
  "path": "graphicInterface/plot_figure.py",
  "blob_sha": "eeb50e13cba928fa1879bec9f1afbed336337ef7",
  "sha256": "bfa4964caa9424895b93a3c92799533b9e108d30ae9cec050bc37491d9ec4bce",
  "lines": 148,
  "bytes": 5481,
  "selection_score": 3,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/rickie95/3DMMRegistration/blob/360f52de184fa063a61e05f184a6de6a42d4f670/graphicInterface/plot_figure.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def __init__(self, parent, model=None, landmarks=True, title=None):"
    },
    {
      "line": 32,
      "text": "def load_model(self, model):"
    },
    {
      "line": 36,
      "text": "def load_data(self, data, color, marker='o', points_size=0.5, label=\"\"):"
    },
    {
      "line": 52,
      "text": "def load_displacement(self):"
    },
    {
      "line": 55,
      "text": "def load_image(self):"
    },
    {
      "line": 61,
      "text": "def show_displacement(self):"
    },
    {
      "line": 65,
      "text": "def clear(self):"
    },
    {
      "line": 68,
      "text": "def draw_data(self, clear=False):"
    },
    {
      "line": 101,
      "text": "def draw(self, clear=False):"
    },
    {
      "line": 107,
      "text": "def restore_model(self):"
    },
    {
      "line": 111,
      "text": "def landmarks(self, l):"
    },
    {
      "line": 114,
      "text": "def set_landmarks_colors(self, colors):"
    },
    {
      "line": 118,
      "text": "def set_data_colors(self, colors):"
    },
    {
      "line": 122,
      "text": "def get_ax(self):"
    },
    {
      "line": 125,
      "text": "def update_plot_callback(self, iteration, error, X, Y, ax):"
    },
    {
      "line": 144,
      "text": "def has_model(self):"
    }
  ]
}

```text

import numpy as np
from PyQt5.QtWidgets import QSizePolicy
from matplotlib import pyplot
from matplotlib.backends.backend_qt5agg import FigureCanvasQTAgg as FigureCanvas

from graphicInterface.console import suppress_stdout_stderr


class PlotFigure(FigureCanvas):

    def __init__(self, parent, model=None, landmarks=True, title=None):
        self.fig, self.ax = pyplot.subplots()
        self.title = title
        FigureCanvas.__init__(self, self.fig)
        FigureCanvas.setSizePolicy(self, QSizePolicy.Expanding, QSizePolicy.Expanding)
        FigureCanvas.updateGeometry(self)
        self.setParent(parent)
        self.draw_landmarks = landmarks
        self.bgImage = None
        self.model = model
        self.drawDisplacement = False
        self.landmarks_colors = None
        self.data_colors = None
        self.scale_width = None
        self.scale_height = None
        self.legend_handlers = []
        self.legend_has_been_drawn = True

        if model is not None:
            self.load_model(model)

    def load_model(self, model):
        self.model = model
        self.drawDisplacement = False

    def load_data(self, data, color, marker='o', points_size=0.5, label=""):
        if data is None or data.shape[0] == 0:
            return
        append = False
        if label != "":
            label += f" ({data.shape[0]})"
            append = True
        max_v = np.max(data[:, 2])
        min_v = np.min(data[:, 2])
        sizes = np.copy(data[:, 2])
        self.sizes = ((sizes + np.abs(min_v)) / np.abs(max_v)) + 0.5
        legend_handler, = self.ax.plot(data[:, 0], data[:, 1], c=color, marker=marker, linestyle='None',
                                       markersize=points_size, label=label)
        if append:
            self.legend_handlers.append(legend_handler)

    def load_displacement(self):
        self.ax.plot(self.model.displacement_map[:, 0], self.model.displacement_map[:, 1])

    def load_image(self):
        if self.bgImage is not None:
            img = pyplot.imread(self.bgImage)
            self.ax.imshow(img, extent=[-self.scale_height * 1.05, self.scale_height * 1.03, -self.scale_height * 1.03,
                                        self.scale_height * 1.05])  # SX DX BOTTOM UP

    def show_displacement(self):
        self.drawDisplacement = True if self.model.displacement_map is not None else False
        self.draw_landmarks = False

    def clear(self):
        self.ax.cla()

    def draw_data(self, clear=False):
        if clear:
            self.ax.cla()
        if self.title is not None:
            self.ax.set_title(self.title)
        if self.model is not None:
            self.ax.autoscale()
            self.ax.set_aspect('equal')
            self.ax.set_xlabel('X axis')
            self.ax.set_ylabel('Y axis')

            self.load_data(self.model.points, self.model.points_color, label="Points")
            if self.model.landmarks is not None:
                self.load_data(self.model.landmarks, self.model.landmarks_color, points_size=5, label="Landmarks")
            try:
                self.load_data(self.model.points[self.model.registration_points], 'y', label="Registration Points")
            except Exception as ex:
                pass

            try:
                self.load_data(self.model.missed_points, self.model.missed_points_color, marker='v',
                               label="Missed Points")
                self.load_data(self.model.missed_landmarks, self.model.missed_landmarks_color, size=5, marker='v',
                               label="Missed Landmarks")
            except Exception as ex:
                pass
            """if len(self.legend_handlers) > 0 and self.legend_has_been_drawn:
                self.ax.legend(handles=self.legend_handlers)
                self.legend_has_been_drawn = False
            """
            self.load_image()
        self.draw()

    def draw(self, clear=False):
        if clear is True:
            self.clear()
        super().draw()
        self.flush_events()

    def restore_model(self):
        self.ax.cla()
        self.draw_data()

    def landmarks(self, l):
        self.draw_landmarks = l

    def set_landmarks_colors(self, colors):
        self.model.landmarks_color = colors
        self.draw_data()

    def set_data_colors(self, colors):
        self.model.points_color = colors
        self.draw_data()

    def get_ax(self):
        return self.ax

    def update_plot_callback(self, iteration, error, X, Y, ax):
        self.ax.cla()
        print(iteration, error)
        self.ax.scatter(Y[:, 0], Y[:, 1], 0.1, c='r')
        self.ax.scatter(X[:, 0], X[:, 1], 0.1, c='b')
        if self.bgImage is not None:
            img = pyplot.imread(self.bgImage)
            self.ax.imshow(img, extent=[-self.model.rangeY/2 * 1.05, self.model.rangeY/2 * 1.03,
                                        -self.model.rangeY/2 * 1.03, self.model.rangeY/2 * 1.05])
        try:
            self.ax.text(0.87, 0.92, 'Iteration: {:d}\nError: {:06.4f}'.format(iteration, error),
                         horizontalalignment='center', verticalalignment='center', transform=self.ax.transAxes,
                         fontsize='x-large')
        except Exception as ex:
            print(ex)
        with suppress_stdout_stderr():
            super().draw()
        self.flush_events()

    def has_model(self):
        if self.model is not None:
            return True

        return False


```
