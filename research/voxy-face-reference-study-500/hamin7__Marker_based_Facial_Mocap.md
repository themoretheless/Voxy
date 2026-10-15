# Facial rendering source scan: hamin7/Marker_based_Facial_Mocap

Repository: https://github.com/hamin7/Marker_based_Facial_Mocap

Pinned source revision: 15f39ee68a77a3fc686f120b6a21e60147afe2ad

Discovery topic: facial mocap

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"rig": ["include/FaceRig.ma"], "face": ["include/FaceRig.ma"]}

## Source file: src/Form1.cs

{
  "path": "src/Form1.cs",
  "blob_sha": "9939b06be2783100d688f4dfc04d3a46f603cfe6",
  "sha256": "c0bfda6f650ed09a6ab198f0e9671579ab5fe07214ea67187d0ad9b54ba1675e",
  "lines": 496,
  "bytes": 19143,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 2
  },
  "url": "https://github.com/hamin7/Marker_based_Facial_Mocap/blob/15f39ee68a77a3fc686f120b6a21e60147afe2ad/src/Form1.cs",
  "function_signature_candidates": [
    {
      "line": 119,
      "text": "private void seri_OPEN_Click(object sender, EventArgs e)"
    },
    {
      "line": 206,
      "text": "private void seri_CLOSE_Click(object sender, EventArgs e)"
    },
    {
      "line": 225,
      "text": "void SPort_DataReceived(object sender, SerialDataReceivedEventArgs e)"
    },
    {
      "line": 387,
      "text": "private void pictureBox1_Click(object sender, EventArgs e)"
    },
    {
      "line": 393,
      "text": "private void StartnNext_Click(object sender, EventArgs e)"
    },
    {
      "line": 427,
      "text": "private void On_timer1(object sender, EventArgs e)"
    },
    {
      "line": 454,
      "text": "private void Form1_FormClosed(object sender, FormClosedEventArgs e)"
    },
    {
      "line": 467,
      "text": "private void SAVE_Click(object sender, EventArgs e)"
    },
    {
      "line": 479,
      "text": "private void CLOSE_Click(object sender, EventArgs e)"
    }
  ]
}

```text

﻿using System;
using System.Collections.Generic;
using System.Drawing;
using System.IO;
using System.IO.Ports;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Windows.Forms;


namespace intern_8ch
{


    public partial class Form1 : Form
    {
        //optitrack trigger 
        TcpClient tcpClient = null;
        NetworkStream clientStream;

        SerialPort sPort;
        static int channels = 1;             
        static int samples = 16;  
        int bufferSize = channels * samples * 9;
        
        static int degree = 1000;        // V -> mV
        static double VREF = 3.83; //4.5//3.83//3.3       // 1LSB= (2*VREF/Gain)/(2^(24))=+FS(2^(23))
        static byte STX = 0x81;
        static byte ETX = 0x82;
        static int PACKET_SIZE = 34;
        static uint Samplerate = 250;
        static uint BUFSIZE = Samplerate * 3;
        bool DataSaveFlag = false;
        bool feFlag = false;
        int count = -1;
        string thisdate, save_name, save_name_path, save_name_path1, save_name_path2;

        static Random random = new Random();
        System.Diagnostics.Stopwatch stopwch = new System.Diagnostics.Stopwatch();
        int trg = 0;        // V -> mV
        int[] idxFE = new int[11];        // V -> mV



        long[] INPUTDATA = new long[PACKET_SIZE + 1];

        long[] EMG_CH1_data_data = new long[BUFSIZE];
        long[] EMG_CH2_data_data = new long[BUFSIZE];
        long[] EMG_CH3_data_data = new long[BUFSIZE];
        long[] EMG_CH4_data_data = new long[BUFSIZE];
        long[] EMG_CH5_data_data = new long[BUFSIZE];
        long[] EMG_CH6_data_data = new long[BUFSIZE];
        long[] EMG_CH7_data_data = new long[BUFSIZE];
        long[] EMG_CH8_data_data = new long[BUFSIZE];

        double[] EMG_CH1_data_draw = new double[BUFSIZE];
        double[] EMG_CH2_data_draw = new double[BUFSIZE];
        double[] EMG_CH3_data_draw = new double[BUFSIZE];
        double[] EMG_CH4_data_draw = new double[BUFSIZE];
        double[] EMG_CH5_data_draw = new double[BUFSIZE];
        double[] EMG_CH6_data_draw = new double[BUFSIZE];
        double[] EMG_CH7_data_draw = new double[BUFSIZE];
        double[] EMG_CH8_data_draw = new double[BUFSIZE];


        long[] LPF_PPG_RED_data = new long[BUFSIZE];
        long[] PPG_RED_data = new long[BUFSIZE];



        private static List<int> GenerateRandom(int count)
        {
            // generate count random values.
            HashSet<int> candidates = new HashSet<int>();
            while (candidates.Count < count)
            {
                // May strike a duplicate.
                candidates.Add(random.Next());
            }

            // load them in to a list.
            List<int> result = new List<int>();
            result.AddRange(candidates);

            // shuffle the results:
            int i = result.Count;
            while (i > 1)
            {
                i--;
                int k = random.Next(i + 1);
                int value = result[k];
                result[k] = result[i];
                result[i] = value;
            }
            return result;
        }

        public Form1()
        {
            InitializeComponent();

            seri_OPEN.Enabled = true;
            seri_CLOSE.Enabled = false;
            CLOSE.Enabled = false;

            cbPORT.BeginUpdate();
            foreach (string comport in SerialPort.GetPortNames())
            {
                cbPORT.Items.Add(comport);
            }
            cbPORT.EndUpdate();

            CheckForIllegalCrossThreadCalls = false;

        }


        private void seri_OPEN_Click(object sender, EventArgs e)
        {
            try
            {
                // EMG 보드 시리얼 포트 여는 코드
                if (null == sPort)
                {
                    sPort = new SerialPort();
                    sPort.DataReceived += new SerialDataReceivedEventHandler(SPort_DataReceived);
                    sPort.PortName = cbPORT.SelectedItem.ToString();
                    sPort.BaudRate = Convert.ToInt32("115200");
                    sPort.DataBits = (int)8;
                    sPort.Parity = Parity.None;
                    sPort.StopBits = StopBits.One;
                    sPort.Open();
                    sPort.Write("T");                    
                }      
                
                if (sPort.IsOpen)
                {
                    seri_OPEN.Enabled = false;
                    seri_CLOSE.Enabled = true;
                }
                else
                {
                    seri_OPEN.Enabled = true;
                    seri_CLOSE.Enabled = false;

                }

                // TCP 포트 열고 스트림 받음
                if(tcpClient == null)
                {
                    tcpClient = new TcpClient();
                    tcpClient.Connect("localhost", 8888);
                    clientStream = tcpClient.GetStream();

                }


                // 표정 설정 하는 코드

                // repeat until all 8 valid random numbers are generated
                var counter = 0;
                do
                {
                    // generate a random number between 1 and 49
                    Random random = new Random();
                    var randomNumber = random.Next(1, idxFE.Length + 1);

                    // if the numbers array doesn't contain the random number
                    // add the random number to the array and increment the counter
                    if (Array.IndexOf(idxFE, randomNumber) == -1)
                    {
                        if (counter == 0)
                            idxFE[counter] = 9;
                        else
                            idxFE[counter] = randomNumber;
                        counter++;
                    }
                } while (counter < idxFE.Length);

                string strFEList = string.Format("INDEX OF FE: {0}", idxFE[count]);

                textBox1.Text = strFEList;

                //// display the first 7 numbers separated by comma
                //for (var i = 0; i < idxFE.Length - 1; i++)
                //{
                //    Console.Write(idxFE[i] + ",");
                //}

                //// display the last number which doesn't have a comma
                //Console.Write(idxFE[idxFE.Length - 1]);


            }
            catch (System.Exception ex)
            {
                MessageBox.Show(ex.Message);
            }


        }



        private void seri_CLOSE_Click(object sender, EventArgs e)
        {
            if (null != sPort)
            {
                if (sPort.IsOpen)
                {
                    sPort.Write("S");
                    sPort.Close();
                    sPort.Dispose();
                    sPort = null;
                }
            }

            seri_OPEN.Enabled = true;
            seri_CLOSE.Enabled = false;
        }



        void SPort_DataReceived(object sender, SerialDataReceivedEventArgs e)
        {
            
            long EMG_CH1_data, EMG_CH2_data, EMG_CH3_data, EMG_CH4_data, EMG_CH5_data, EMG_CH6_data, EMG_CH7_data, EMG_CH8_data;

            

            while (sPort.BytesToRead > 0)
            {
                //optitrack trigger 
                byte marker = 0;
                if (tcpClient != null && clientStream != null)
                {
                    byte[] bytes = new byte[3];
                    clientStream.Read(bytes, 0, 3);
                    marker = bytes[1];

                }

                for (int ii = 0; ii < PACKET_SIZE; ii++)
                {
                    INPUTDATA[ii] = INPUTDATA[ii + 1];
                }

                long bybuf = sPort.ReadByte();
                INPUTDATA[PACKET_SIZE] = bybuf;


                if (INPUTDATA[0] == STX && INPUTDATA[PACKET_SIZE - 1] == ETX)
                {
                   

                    //DATA Recieve
                    EMG_CH1_data = ((int)INPUTDATA[1] << 21) + ((int)INPUTDATA[2] << 14) + ((int)INPUTDATA[3] << 7) + ((int)INPUTDATA[4]);
                    EMG_CH2_data = ((int)INPUTDATA[5] << 21) + ((int)INPUTDATA[6] << 14) + ((int)INPUTDATA[7] << 7) + ((int)INPUTDATA[8]);
                    EMG_CH3_data = ((int)INPUTDATA[9] << 21) + ((int)INPUTDATA[10] << 14) + ((int)INPUTDATA[11] << 7) + ((int)INPUTDATA[12]);
                    EMG_CH4_data = ((int)INPUTDATA[13] << 21) + ((int)INPUTDATA[14] << 14) + ((int)INPUTDATA[15] << 7) + ((int)INPUTDATA[16]);
                    EMG_CH5_data = ((int)INPUTDATA[17] << 21) + ((int)INPUTDATA[18] << 14) + ((int)INPUTDATA[19] << 7) + ((int)INPUTDATA[20]);
                    EMG_CH6_data = ((int)INPUTDATA[21] << 21) + ((int)INPUTDATA[22] << 14) + ((int)INPUTDATA[23] << 7) + ((int)INPUTDATA[24]);
                    EMG_CH7_data = ((int)INPUTDATA[25] << 21) + ((int)INPUTDATA[26] << 14) + ((int)INPUTDATA[27] << 7) + ((int)INPUTDATA[28]);
                    EMG_CH8_data = ((int)INPUTDATA[29] << 21) + ((int)INPUTDATA[30] << 14) + ((int)INPUTDATA[31] << 7) + ((int)INPUTDATA[32]);


                    //convert EEG Complement
                    EMG_CH1_data = (EMG_CH1_data & 0x7FFFFF) - (EMG_CH1_data & 0x800000);//CH_1_two = (CH_1 & 0x7fffff) - (CH_1 & 0x800000);
                    EMG_CH2_data = (EMG_CH2_data & 0x7FFFFF) - (EMG_CH2_data & 0x800000);
                    EMG_CH3_data = (EMG_CH3_data & 0x7FFFFF) - (EMG_CH3_data & 0x800000);//CH_1_two = (CH_1 & 0x7fffff) - (CH_1 & 0x800000);
                    EMG_CH4_data = (EMG_CH4_data & 0x7FFFFF) - (EMG_CH4_data & 0x800000);
                    EMG_CH5_data = (EMG_CH5_data & 0x7FFFFF) - (EMG_CH5_data & 0x800000);//CH_1_two = (CH_1 & 0x7fffff) - (CH_1 & 0x800000);
                    EMG_CH6_data = (EMG_CH6_data & 0x7FFFFF) - (EMG_CH6_data & 0x800000);
                    EMG_CH7_data = (EMG_CH7_data & 0x7FFFFF) - (EMG_CH7_data & 0x800000);//CH_1_two = (CH_1 & 0x7fffff) - (CH_1 & 0x800000);
                    EMG_CH8_data = (EMG_CH8_data & 0x7FFFFF) - (EMG_CH8_data & 0x800000);

                    //convert EEG voltage 24 gain

                    EMG_CH1_data = (long)(EMG_CH1_data * VREF * degree) / (8388607);
                    EMG_CH2_data = (long)(EMG_CH2_data * VREF * degree) / (8388607);
                    EMG_CH3_data = (long)(EMG_CH3_data * VREF * degree) / (8388607);
                    EMG_CH4_data = (long)(EMG_CH4_data * VREF * degree) / (8388607);
                    EMG_CH5_data = (long)(EMG_CH5_data * VREF * degree) / (8388607);
                    EMG_CH6_data = (long)(EMG_CH6_data * VREF * degree) / (8388607);
                    EMG_CH7_data = (long)(EMG_CH7_data * VREF * degree) / (8388607);
                    EMG_CH8_data = (long)(EMG_CH8_data * VREF * degree) / (8388607);


                    //Buffering
                    for (int j = 0; j < (BUFSIZE - 1); j++)
                    {

                        EMG_CH1_data_data[j] = EMG_CH1_data_data[j + 1];
                        EMG_CH2_data_data[j] = EMG_CH2_data_data[j + 1];
                        EMG_CH3_data_data[j] = EMG_CH3_data_data[j + 1];
                        EMG_CH4_data_data[j] = EMG_CH4_data_data[j + 1];
                        EMG_CH5_data_data[j] = EMG_CH5_data_data[j + 1];
                        EMG_CH6_data_data[j] = EMG_CH6_data_data[j + 1];
                        EMG_CH7_data_data[j] = EMG_CH7_data_data[j + 1];
                        EMG_CH8_data_data[j] = EMG_CH8_data_data[j + 1];


                    }


                    EMG_CH1_data_data[BUFSIZE - 1] = EMG_CH1_data;
                    EMG_CH2_data_data[BUFSIZE - 1] = EMG_CH2_data;
                    EMG_CH3_data_data[BUFSIZE - 1] = EMG_CH3_data;
                    EMG_CH4_data_data[BUFSIZE - 1] = EMG_CH4_data;
                    EMG_CH5_data_data[BUFSIZE - 1] = EMG_CH5_data;
                    EMG_CH6_data_data[BUFSIZE - 1] = EMG_CH6_data;
                    EMG_CH7_data_data[BUFSIZE - 1] = EMG_CH7_data;
                    EMG_CH8_data_data[BUFSIZE - 1] = EMG_CH8_data;

                    if (DataSaveFlag)
                    {
                        int trg;

                        if ((count >= 0 && count < 11) && (stopwch.ElapsedMilliseconds >= 3000))
                            trg = idxFE[count];
                        else if (count >=11)
                            trg = 0;
                        else
                            trg = 0;


                        
                        string dataline = string.Format("{0},{1},{2},{3},{4},{5},{6},{7},{8},{9}", (float)EMG_CH1_data, (float)EMG_CH2_data, (float)EMG_CH3_data, (float)EMG_CH4_data, (float)EMG_CH5_data, (float)EMG_CH6_data, (float)EMG_CH7_data, (float)EMG_CH8_data, trg, marker);
                        Console.WriteLine(dataline);

                        StreamWriter sw = new StreamWriter(save_name, true);
                        sw.WriteLine(dataline);
                        sw.Close();


                    }

                    string str;

                    if (stopwch.ElapsedMilliseconds >= 1000 && stopwch.ElapsedMilliseconds < 2000)
                    {
                        str = "Wait (2 s left)";
                        textBox1.Text = str;
                    }
                    else if (stopwch.ElapsedMilliseconds >= 2000 && stopwch.ElapsedMilliseconds < 3000)
                    {
                        str = "Wait (1 s left)";
                        textBox1.Text = str;
                    }
                    else if (stopwch.ElapsedMilliseconds >= 3000 && stopwch.ElapsedMilliseconds < 4000)
                    {
                        str = "Make a Facial Expression (1 s)";
                        textBox1.Text = str;
                    }

                    if (feFlag && stopwch.ElapsedMilliseconds >= 3000)
                    {
                        

                        if (stopwch.ElapsedMilliseconds >= 4000 && stopwch.ElapsedMilliseconds < 5000)
                        {
                            str = "Make a Facial Expression (2 s)";
                            textBox1.Text = str;
                        }
                        else if (stopwch.ElapsedMilliseconds >= 5000 && stopwch.ElapsedMilliseconds < 6000)
                        {
                            str = "Make a Facial Expression (3 s)";
                            textBox1.Text = str;
                        }
                        else if (stopwch.ElapsedMilliseconds >= 6000 && stopwch.ElapsedMilliseconds < 7000)
                        {
                            str = "Rest";
                            textBox1.Text = str;
                            feFlag = false;
                            StartnNext.Enabled = true;
                            stopwch.Reset();
                        }


                    }
                }
            }

        }

        private void pictureBox1_Click(object sender, EventArgs e)
        {


        }

        private void StartnNext_Click(object sender, EventArgs e)
        {
            
            feFlag = true;
            count++;
            StartnNext.Enabled = false;
            // 실험 끝났을 때
            if (count == 11)
            {
                //DataSaveFlag = false;
                timer1.Stop();
                seri_CLOSE.Enabled = false;
                StartnNext.Enabled = false;
            }
            else
            {
                // 사진 파일 선택
                string strFormattedDate = string.Format("resize_{0}.png", idxFE[count]);
                string file_fullpath1 = System.IO.Path.Combine(System.IO.Directory.GetParent(Application.StartupPath).FullName, "pics", strFormattedDate);

                // 사진 바꾸기
                pictureBox1.Image = Bitmap.FromFile(file_fullpath1);
                pictureBox1.Visible = true; ;
                //pictureBox1.ImageLocation.Normalize();


                // 시간 저장 및 초기화
                if (!stopwch.IsRunning)
                    stopwch.Start();
            }
            

        }

        private void On_timer1(object sender, EventArgs e)
        {


            for (int i = 0; i < BUFSIZE; i++)
            {
                EMG_CH1_data_draw[i] = EMG_CH1_data_data[i];
                EMG_CH2_data_draw[i] = EMG_CH2_data_data[i];
                EMG_CH3_data_draw[i] = EMG_CH3_data_data[i];
                EMG_CH4_data_draw[i] = EMG_CH4_data_data[i];
                EMG_CH5_data_draw[i] = EMG_CH5_data_data[i];
                EMG_CH6_data_draw[i] = EMG_CH6_data_data[i];
                EMG_CH7_data_draw[i] = EMG_CH7_data_data[i];
                EMG_CH8_data_draw[i] = EMG_CH8_data_data[i];

            }

            scope1.Channels[0].Data.SetYData(EMG_CH1_data_draw, 0, BUFSIZE);
            scope2.Channels[0].Data.SetYData(EMG_CH2_data_draw, 0, BUFSIZE);
            scope3.Channels[0].Data.SetYData(EMG_CH3_data_draw, 0, BUFSIZE);
            scope4.Channels[0].Data.SetYData(EMG_CH4_data_draw, 0, BUFSIZE);
            scope5.Channels[0].Data.SetYData(EMG_CH5_data_draw, 0, BUFSIZE);
            scope6.Channels[0].Data.SetYData(EMG_CH6_data_draw, 0, BUFSIZE);
            scope7.Channels[0].Data.SetYData(EMG_CH7_data_draw, 0, BUFSIZE);
            scope8.Channels[0].Data.SetYData(EMG_CH8_data_draw, 0, BUFSIZE);
        }

        private void Form1_FormClosed(object sender, FormClosedEventArgs e)
        {
            if (null != sPort)
            {
                if (sPort.IsOpen)
                {
                    sPort.Close();
                    sPort.Dispose();
                    sPort = null;
                }
            }
        }

        private void SAVE_Click(object sender, EventArgs e)
        {
            SAVE.Enabled = false;
            CLOSE.Enabled = true;

            thisdate = DateTime.Now.ToString("yyyy년MM월dd일hh시mm분ss초");
            save_name = thisdate + ".txt";
            FileStream fsa = File.Create(save_name);
            fsa.Close();
            DataSaveFlag = true;
        }

        private void CLOSE_Click(object sender, EventArgs e)
        {
            if (DataSaveFlag)
            {
                DataSaveFlag = false;
                CLOSE.Enabled = false;
                SAVE.Enabled = false;
                seri_CLOSE.Enabled = false;
                StreamWriter sw = new StreamWriter(save_name, true);
                thisdate = DateTime.Now.ToString("yyyy년MM월dd일hh시mm분ss초");
                sw.WriteLine("End Time : " + thisdate);
                sw.Close();
            }

        }
    }
}



```

## Source file: src/setting.py

{
  "path": "src/setting.py",
  "blob_sha": "82569a02e89cc3c398e3808fd88cea6001a179dc",
  "sha256": "7adabfdba381b21d6d4faafb321bd643d6f22ae0814132cd5b0f3e5f10921be5",
  "lines": 534,
  "bytes": 22868,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 23,
    "jaw": 24,
    "teeth": 8,
    "deform": 3,
    "rig": 3,
    "face": 3,
    "lip": 105
  },
  "url": "https://github.com/hamin7/Marker_based_Facial_Mocap/blob/15f39ee68a77a3fc686f120b6a21e60147afe2ad/src/setting.py",
  "function_signature_candidates": [
    {
      "line": 31,
      "text": "def edit_cell(row, column, value):"
    },
    {
      "line": 34,
      "text": "def startrealtimeexp(*args):"
    },
    {
      "line": 51,
      "text": "def expTrackerWindow():"
    },
    {
      "line": 191,
      "text": "def deformface():"
    },
    {
      "line": 506,
      "text": "def portData(arg):"
    },
    {
      "line": 528,
      "text": "def deactivateCommandPort(host, port):"
    }
  ]
}

```text

import maya.cmds as cmds
import os
import random
import pymel.core as pm
import socket
import sys
import maya.mel as mm
import csv

recstart = 0
selectedBlend = ""
dataarray = []
gnumcurrenttime = 1

"""
menulabelarr = ['Brow Left UP','Brow Left Down','Brow Right UP','Brow Right Down','Brow Centering','Brow outer left down','Brow outer right down','Eye Close Left','Eye Close Right','Mouse Open',
                'Mouse Left Smile','Mouse Right Smile','Mouse Left Spread','Mouse Right Spread','Mouse Left Frawn','Mouse Right Frawn','Mouse Left Centering','Mouse Right Centering','Cheek Left UP',
                'Cheek Right UP']
"""


# Our mel global proc.
melproc = """
global proc portData(string $arg){
    python(("portData(\\"" + $arg + "\\")"));
}
"""

mm.eval(melproc)

def edit_cell(row, column, value):
    return 1

def startrealtimeexp(*args):
    global recstart

    #numrow = cmds.scriptTable('scrtable', query=True, rows=True)
    if len(selectedBlend) >= 0:
        if recstart == 0:
            recstart = 1
            cmds.button( 'realtimecomm' ,edit=True, label = 'Stop Real Time Expression' )
	    #Start Comm
            cmds.commandPort(name="127.0.0.1:7777", echoOutput=False, noreturn=False,prefix="portData", returnNumCommands=True)
            cmds.commandPort(name=":7777", echoOutput=False, noreturn=False,prefix="portData", returnNumCommands=True)
        else:
            recstart = 0
            cmds.button( 'realtimecomm' ,edit=True, label = 'Start Real Time Expression' )
	    #Stop Comm
            deactivateCommandPort('127.0.0.1', '7777')

def expTrackerWindow():
    if cmds.window('expTrackerWindow', exists = True):
        cmds.deleteUI('expTrackerWindow')

    #window def
    cmds.window('expTrackerWindow',widthHeight=(900,400),title='Perseus_Boy_Facial_Mocap-conelab',minimizeButton=False,maximizeButton=False,resizeToFitChildren = True, sizeable = True)
    #cmds.rowColumnLayout(numberOfColumns=3,columnWidth=[(1,300),(2,300),(3,300)],backgroundColor=[200,200,0])

    cmds.columnLayout('temp3', width=900)
    cmds.rowColumnLayout(numberOfColumns=5,columnWidth=[(1,200),(2,50),(3,200),(4,150),(5,300)])

    cmds.text( label='' )
    cmds.text( label='' )
    cmds.text( label='' )
    cmds.text( label='' )
    cmds.text( label='' )

    #cmds.button( label = 'Save preset', command = savepresetfile,backgroundColor=[0.3412,0.8196,0.7882] )
    cmds.text( label='' )
    #cmds.button( label = 'Load preset', command = loadpresetfile,backgroundColor=[0.3412,0.8196,0.7882] )
    cmds.text( label='' )
    cmds.button( 'realtimecomm', label = 'Start Real Time Expression', command = startrealtimeexp ,backgroundColor=[0.9294,0.3294,0.5216])
    
    #cmds.button(label='close', command=('cmds.deleteUI(\"' + window + '\", window=True)'), backgroundColor=[0.9294,0.3294,0.5216])

    cmds.text( label='' )
    cmds.text( label='' )
    cmds.text( label='' )
    cmds.text( label='' )
    cmds.text( label='' )

    cmds.showWindow( 'expTrackerWindow' )

expTrackerWindow()

# 0 ~ 2
#PRE_name_Nose_ctrl_Z = 0.0088093
PRE_name_Nose_ctrl_Y = 0.267482
PRE_name_Nose_ctrl_X = -0.36317

# 3 ~ 5
#PRE_name_downLip_ctrl_Z = 0.85043
PRE_name_downLip_ctrl_Y = 0.234867
PRE_name_downLip_ctrl_X = -0.371

# 6 ~ 8
#PRE_name_l_downLip_ctrl_Z = 0.045024
PRE_name_l_downLip_ctrl_Y = 0.177909
PRE_name_l_downLip_ctrl_X = -0.35571

# 9 ~ 11
#PRE_name_r_downLip_ctrl_Z = 0.076658
PRE_name_r_downLip_ctrl_Y = 0.186614
PRE_name_r_downLip_ctrl_X = -0.38361

# 12 ~ 14
#PRE_name_l_up_cheek_ctrl_Z = 0.119772
PRE_name_l_up_cheek_ctrl_Y = 0.185076
PRE_name_l_up_cheek_ctrl_X = -0.35978

# 15 ~ 17
#PRE_name_l_cheek_ctrl_Z = 0.075271
PRE_name_l_cheek_ctrl_Y = 0.114699
PRE_name_l_cheek_ctrl_X = -0.3695

# 18 ~ 20
#PRE_name_l_Nose_ctrl_Z = 0.145003
PRE_name_l_Nose_ctrl_Y = 0.267482
PRE_name_l_Nose_ctrl_X = -0.36317

# 21 ~ 23
#PRE_name_l_Lip_ctrl_Z = 0.13379
PRE_name_l_Lip_ctrl_Y = 0.234867
PRE_name_l_Lip_ctrl_X = -0.371

# 24 ~ 26
#PRE_name_l_jaw_cheek_ctrl_Z = 0.112907
PRE_name_l_jaw_cheek_ctrl_Y = 0.177909
PRE_name_l_jaw_cheek_ctrl_X = -0.35571

# 27 ~ 29
#PRE_name_l_nose_cheek_ctrl_Z = 0.146252
PRE_name_l_nose_cheek_ctrl_Y = 0.186614
PRE_name_l_nose_cheek_ctrl_X = -0.38361

# 30 ~ 32
#PRE_name_r_up_cheek_ctrl_Z = 0.13576
PRE_name_r_up_cheek_ctrl_Y = 0.185076
PRE_name_r_up_cheek_ctrl_X = -0.35978

# 33 ~ 35
#PRE_name_r_cheek_ctrl_Z = 0.119672
PRE_name_r_cheek_ctrl_Y = 0.114699
PRE_name_r_cheek_ctrl_X = -0.3695

# 36 ~ 38
#PRE_name_r_Nose_ctrl_Z = 0.143902
PRE_name_r_Nose_ctrl_Y = 0.267482
PRE_name_r_Nose_ctrl_X = -0.36317

# 39 ~ 41
#PRE_name_r_Lip_ctrl_Z = 0.134883
PRE_name_r_Lip_ctrl_Y = 0.234867
PRE_name_r_Lip_ctrl_X = -0.371

# 42 ~ 44
#PRE_name_r_jaw_cheek_ctrl_Z = 0.109223
PRE_name_r_jaw_cheek_ctrl_Y = 0.177909
PRE_name_r_jaw_cheek_ctrl_X = -0.35571

# 45 ~ 47
#PRE_name_r_nose_cheek_ctrl_Z = 0.142525
PRE_name_r_nose_cheek_ctrl_Y = 0.186614
PRE_name_r_nose_cheek_ctrl_X = -0.38361

# 48 ~ 50
#PRE_name_l_down_eye_border_ctrl_Z = 0.143587
PRE_name_l_down_eye_border_ctrl_Y = 0.185076
PRE_name_l_down_eye_border_ctrl_X = -0.35978

# 51 ~ 53
#PRE_name_r_down_eye_border_ctrl_Z = 0.139288
PRE_name_r_down_eye_border_ctrl_Y = 0.114699
PRE_name_r_down_eye_border_ctrl_X = -0.3695

# 54 ~ 56
#PRE_name_upLip_ctrl_Z = 0.152267
PRE_name_upLip_ctrl_Y = 0.267482
PRE_name_upLip_ctrl_X = -0.36317

# 57 ~ 59
#PRE_name_l_upLip_ctrl_Z = 0.147181
PRE_name_l_upLip_ctrl_Y = 0.234867
PRE_name_l_upLip_ctrl_X = -0.371

# 60 ~ 62
#PRE_name_r_upLip_ctrl_Z = 0.146955
PRE_name_r_upLip_ctrl_Y = 0.177909
PRE_name_r_upLip_ctrl_X = -0.35571

def deformface():
    global dataarray
    if len(dataarray) < 0:
        # dataarray의 인자의 첫 세개는 목뼈의 x,y,z좌표 이므로 이것이 없다면 facial_mocap의 의미 없으므로 0을 리턴.
    	return 0

    #all_rows = cmds.scriptTable('scrtable', query=True, rows=True)
    global recstart
    global gnumcurrenttime
    
    global strengthX
    global strengthY
    #global strengthZ
    
    strengthX = 75
    strengthY = 45
    #strengthZ = 50
    
    all_rows = 1
    if all_rows > 0 and recstart == 1:
        
        # name_Nose_ctrl (dataarray[0] ~ dataarray[2])
            
        global PRE_name_Nose_ctrl_X
        global PRE_name_Nose_ctrl_Y
        #global PRE_name_Nose_ctrl_Z
        
        pm.move((float(dataarray[2]) - float(PRE_name_Nose_ctrl_X)) * strengthX, (float(dataarray[1]) - float(PRE_name_Nose_ctrl_Y)) * strengthY, 'name_Nose_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )
        
        #PRE_name_Nose_ctrl_Z = -float(dataarray[0])
        PRE_name_Nose_ctrl_Y = dataarray[1]
        PRE_name_Nose_ctrl_X = dataarray[2]
        
        # name_downLip_ctrl (dataarray[3] ~ dataarray[5])
        # name_down_teeth_ctrl (dataarray[3] ~ dataarray[5])
        # name_jaw_ctrl (dataarray[3] ~ dataarray[5])
            
        global PRE_name_downLip_ctrl_X
        global PRE_name_downLip_ctrl_Y
        #global PRE_name_downLip_ctrl_Z
        
        pm.move(float((float(dataarray[5]) - float(PRE_name_downLip_ctrl_X)) * strengthX) * (-1), float((float(dataarray[4]) - float(PRE_name_downLip_ctrl_Y)) * strengthY) * (-1), 'name_downLip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )
        # 치아는 입술보단 덜 움직이므로 x 좌표 움직임에 0.8 곱함.
        pm.move(float((float(dataarray[5]) - float(PRE_name_downLip_ctrl_X)) * strengthX) * (-0.5), float((float(dataarray[4]) - float(PRE_name_downLip_ctrl_Y)) * strengthY) * (0.8), 'name_down_teeth_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )
        # 턱도 아랫입술과 같이 움직임
        pm.move(float((float(dataarray[5]) - float(PRE_name_downLip_ctrl_X)) * strengthX) * (-0.5), float((float(dataarray[4]) - float(PRE_name_downLip_ctrl_Y)) * strengthY) * (0.8), 'name_jaw_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_downLip_ctrl_Z = -float(dataarray[3])
        PRE_name_downLip_ctrl_Y = dataarray[4]
        PRE_name_downLip_ctrl_X = dataarray[5]

        ############################################################################################
        # name_down_teeth_ctrl (dataarray[3] ~ dataarray[5])
            
        #global PRE_name_downLip_ctrl_X
        #global PRE_name_downLip_ctrl_Y
        #global PRE_name_downLip_ctrl_Z
        
        # 치아는 입술보단 덜 움직이므로 x 좌표 움직임에 0.7 곱함.
        #pm.move(float((float(dataarray[5]) - float(PRE_name_downLip_ctrl_X)) * strengthX) * (-0.7), float((float(dataarray[4]) - float(PRE_name_downLip_ctrl_Y)) * strengthY) * (-1), 'name_down_teeth_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_downLip_ctrl_Z = -float(dataarray[3])
        #PRE_name_downLip_ctrl_Y = dataarray[4]
        #PRE_name_downLip_ctrl_X = dataarray[5]
        ############################################################################################
        
        # name_l_downLip_ctrl (dataarray[6] ~ dataarray[8])
        
        global PRE_name_l_downLip_ctrl_X
        global PRE_name_l_downLip_ctrl_Y
        #global PRE_name_l_downLip_ctrl_Z
        
        pm.move(float((float(dataarray[8]) - float(PRE_name_l_downLip_ctrl_X)) * strengthX) * (-1), float((float(dataarray[7]) - float(PRE_name_l_downLip_ctrl_Y)) * strengthY) * (-1), 'name_l_downLip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )


        #PRE_name_l_downLip_ctrl_Z = -float(dataarray[6])
        PRE_name_l_downLip_ctrl_Y = dataarray[7]
        PRE_name_l_downLip_ctrl_X = dataarray[8]
        
        # name_r_downLip_ctrl (dataarray[9] ~ dataarray[11])
        
        global PRE_name_r_downLip_ctrl_X
        global PRE_name_r_downLip_ctrl_Y
        #global PRE_name_r_downLip_ctrl_Z
        
        pm.move((float(dataarray[11]) - float(PRE_name_r_downLip_ctrl_X)) * strengthX, float((float(dataarray[10]) - float(PRE_name_r_downLip_ctrl_Y)) * strengthY) * (-1), 'name_r_downLip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_r_downLip_ctrl_Z = -float(dataarray[9])
        PRE_name_r_downLip_ctrl_Y = dataarray[10]
        PRE_name_r_downLip_ctrl_X = dataarray[11]
        
        # name_l_up_cheek_ctrl (dataarray[12] ~ dataarray[14])
        
        global PRE_name_l_up_cheek_ctrl_X
        global PRE_name_l_up_cheek_ctrl_Y
        #global PRE_name_l_up_cheek_ctrl_Z
        
        pm.move((float(dataarray[14]) - float(PRE_name_l_up_cheek_ctrl_X)) * strengthX, (float(dataarray[13]) - float(PRE_name_l_up_cheek_ctrl_Y)) * strengthY, 'name_l_up_cheek_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_l_up_cheek_ctrl_Z = -float(dataarray[12])
        PRE_name_l_up_cheek_ctrl_Y = dataarray[13]
        PRE_name_l_up_cheek_ctrl_X = dataarray[14]
        
        # name_l_cheek_ctrl (dataarray[15] ~ dataarray[17])
        
        global PRE_name_l_cheek_ctrl_X
        global PRE_name_l_cheek_ctrl_Y
        #global PRE_name_l_cheek_ctrl_Z
        
        pm.move((float(dataarray[17]) - float(PRE_name_l_cheek_ctrl_X)) * strengthX, (float(dataarray[16]) - float(PRE_name_l_cheek_ctrl_Y)) * strengthY, 'name_l_cheek_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )
        
        #PRE_name_l_cheek_ctrl_Z = -float(dataarray[15])
        PRE_name_l_cheek_ctrl_Y = dataarray[16]
        PRE_name_l_cheek_ctrl_X = dataarray[17]
        
        # name_l_Nose_ctrl (dataarray[18] ~ dataarray[20])
        
        global PRE_name_l_Nose_ctrl_X
        global PRE_name_l_Nose_ctrl_Y
        #global PRE_name_l_Nose_ctrl_Z
        
        pm.move((float(dataarray[20]) - float(PRE_name_l_Nose_ctrl_X)) * strengthX, (float(dataarray[19]) - float(PRE_name_l_Nose_ctrl_Y)) * strengthY, 'name_l_Nose_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_l_Nose_ctrl_Z = -float(dataarray[18])
        PRE_name_l_Nose_ctrl_Y = dataarray[19]
        PRE_name_l_Nose_ctrl_X = dataarray[20]
        
        # name_l_Lip_ctrl (dataarray[21] ~ dataarray[23])
        
        global PRE_name_l_Lip_ctrl_X
        global PRE_name_l_Lip_ctrl_Y
        #global PRE_name_l_Lip_ctrl_Z
        
        pm.move(float((float(dataarray[23]) - float(PRE_name_l_Lip_ctrl_X)) * strengthX) * (-1), (float(dataarray[22]) - float(PRE_name_l_Lip_ctrl_Y)) * strengthY, 'name_l_Lip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_l_Lip_ctrl_Z = -float(dataarray[21])
        PRE_name_l_Lip_ctrl_Y = dataarray[22]
        PRE_name_l_Lip_ctrl_X = dataarray[23]
        
        # name_l_jaw_cheek_ctrl (dataarray[24] ~ dataarray[26])
        
        global PRE_name_l_jaw_cheek_ctrl_X
        global PRE_name_l_jaw_cheek_ctrl_Y
        #global PRE_name_l_jaw_cheek_ctrl_Z

        pm.move((float(dataarray[26]) - float(PRE_name_l_jaw_cheek_ctrl_X)) * strengthX, (float(dataarray[25]) - float(PRE_name_l_jaw_cheek_ctrl_Y)) * strengthY, 'name_l_jaw_cheek_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_l_jaw_cheek_ctrl_Z = -float(dataarray[24])
        PRE_name_l_jaw_cheek_ctrl_Y = dataarray[25]
        PRE_name_l_jaw_cheek_ctrl_X = dataarray[26]
        
        # name_l_nose_cheek_ctrl (dataarray[27] ~ dataarray[29])
        
        global PRE_name_l_nose_cheek_ctrl_X
        global PRE_name_l_nose_cheek_ctrl_Y
        #global PRE_name_l_nose_cheek_ctrl_Z
        
        pm.move((float(dataarray[29]) - float(PRE_name_l_nose_cheek_ctrl_X)) * strengthX, (float(dataarray[28]) - float(PRE_name_l_nose_cheek_ctrl_Y)) * strengthY, 'name_l_nose_cheek_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_l_nose_cheek_ctrl_Z = -float(dataarray[27])
        PRE_name_l_nose_cheek_ctrl_Y = dataarray[28]
        PRE_name_l_nose_cheek_ctrl_X = dataarray[29]
        
        # name_r_up_cheek_ctrl (dataarray[30] ~ dataarray[32])
        
        global PRE_name_r_up_cheek_ctrl_X
        global PRE_name_r_up_cheek_ctrl_Y
        #global PRE_name_r_up_cheek_ctrl_Z
        
        pm.move((float(dataarray[32]) - float(PRE_name_r_up_cheek_ctrl_X)) * strengthX, (float(dataarray[31]) - float(PRE_name_r_up_cheek_ctrl_Y)) * strengthY, 'name_r_up_cheek_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_r_up_cheek_ctrl_Z = -float(dataarray[30])
        PRE_name_r_up_cheek_ctrl_Y = dataarray[31]
        PRE_name_r_up_cheek_ctrl_X = dataarray[32]
        
        # name_r_cheek_ctrl (dataarray[33] ~ dataarray[35])
        
        global PRE_name_r_cheek_ctrl_X
        global PRE_name_r_cheek_ctrl_Y
        #global PRE_name_r_cheek_ctrl_Z
        
        pm.move((float(dataarray[35]) - float(PRE_name_r_cheek_ctrl_X)) * strengthX, (float(dataarray[34]) - float(PRE_name_r_cheek_ctrl_Y)) * strengthY, 'name_r_cheek_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_r_cheek_ctrl_Z = -float(dataarray[33])
        PRE_name_r_cheek_ctrl_Y = dataarray[34]
        PRE_name_r_cheek_ctrl_X = dataarray[35]
        
        # name_r_Nose_ctrl (dataarray[36] ~ dataarray[38])
        
        global PRE_name_r_Nose_ctrl_X
        global PRE_name_r_Nose_ctrl_Y
        #global PRE_name_r_Nose_ctrl_Z
        
        pm.move((float(dataarray[38]) - float(PRE_name_r_Nose_ctrl_X)) * strengthX, (float(dataarray[37]) - float(PRE_name_r_Nose_ctrl_Y)) * strengthY, 'name_r_Nose_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_r_Nose_ctrl_Z = -float(dataarray[36])
        PRE_name_r_Nose_ctrl_Y = dataarray[37]
        PRE_name_r_Nose_ctrl_X = dataarray[38]
        
        # name_r_Lip_ctrl (dataarray[39] ~ dataarray[41])
        
        global PRE_name_r_Lip_ctrl_X
        global PRE_name_r_Lip_ctrl_Y
        #global PRE_name_r_Lip_ctrl_Z
        
        pm.move((float(dataarray[41]) - float(PRE_name_r_Lip_ctrl_X)) * strengthX, (float(dataarray[40]) - float(PRE_name_r_Lip_ctrl_Y)) * strengthY, 'name_r_Lip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_r_Lip_ctrl_Z = -float(dataarray[39])
        PRE_name_r_Lip_ctrl_Y = dataarray[40]
        PRE_name_r_Lip_ctrl_X = dataarray[41]
        
        # name_r_jaw_cheek_ctrl (dataarray[42] ~ dataarray[44])
        
        global PRE_name_r_jaw_cheek_ctrl_X
        global PRE_name_r_jaw_cheek_ctrl_Y
        #global PRE_name_r_jaw_cheek_ctrl_Z
        
        pm.move((float(dataarray[44]) - float(PRE_name_r_jaw_cheek_ctrl_X)) * strengthX, (float(dataarray[43]) - float(PRE_name_r_jaw_cheek_ctrl_Y)) * strengthY, 'name_r_jaw_cheek_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_r_jaw_cheek_ctrl_Z = -float(dataarray[42])
        PRE_name_r_jaw_cheek_ctrl_Y = dataarray[43]
        PRE_name_r_jaw_cheek_ctrl_X = dataarray[44]
        
        # name_r_nose_cheek_ctrl (dataarray[45] ~ dataarray[47])
        
        global PRE_name_r_nose_cheek_ctrl_X
        global PRE_name_r_nose_cheek_ctrl_Y
        #global PRE_name_r_nose_cheek_ctrl_Z
        
        pm.move((float(dataarray[47]) - float(PRE_name_r_nose_cheek_ctrl_X)) * strengthX, (float(dataarray[46]) - float(PRE_name_r_nose_cheek_ctrl_Y)) * strengthY, 'name_r_nose_cheek_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_r_nose_cheek_ctrl_Z = -float(dataarray[45])
        PRE_name_r_nose_cheek_ctrl_Y = dataarray[46]
        PRE_name_r_nose_cheek_ctrl_X = dataarray[47]
        
        # name_l_down_eye_border_ctrl (dataarray[48] ~ dataarray[50])
        
        global PRE_name_l_down_eye_border_ctrl_X
        global PRE_name_l_down_eye_border_ctrl_Y
        #global PRE_name_l_down_eye_border_ctrl_Z
        
        pm.move((float(dataarray[50]) - float(PRE_name_l_down_eye_border_ctrl_X)) * strengthX, (float(dataarray[49]) - float(PRE_name_l_down_eye_border_ctrl_Y)) * strengthY, 'name_l_down_eye_border_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_l_down_eye_border_ctrl_Z = -float(dataarray[48])
        PRE_name_l_down_eye_border_ctrl_Y = dataarray[49]
        PRE_name_l_down_eye_border_ctrl_X = dataarray[50]
        
        # name_r_down_eye_border_ctrl (dataarray[51] ~ dataarray[53])
        
        global PRE_name_r_down_eye_border_ctrl_X
        global PRE_name_r_down_eye_border_ctrl_Y
        #global PRE_name_r_down_eye_border_ctrl_Z
        
        pm.move((float(dataarray[53]) - float(PRE_name_r_down_eye_border_ctrl_X)) * strengthX, (float(dataarray[52]) - float(PRE_name_r_down_eye_border_ctrl_Y)) * strengthY, 'name_r_down_eye_border_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_r_down_eye_border_ctrl_Z = -float(dataarray[51])
        PRE_name_r_down_eye_border_ctrl_Y = dataarray[52]
        PRE_name_r_down_eye_border_ctrl_X = dataarray[53]
        
        # name_upLip_ctrl (dataarray[54] ~ dataarray[56])
        # name_up_teeth_ctrl (dataarray[54] ~ dataarray[56])
        
        global PRE_name_upLip_ctrl_X
        global PRE_name_upLip_ctrl_Y
        #global PRE_name_upLip_ctrl_Z
        
        pm.move(float((float(dataarray[56]) - float(PRE_name_upLip_ctrl_X)) * strengthX) * (-1), (float(dataarray[55]) - float(PRE_name_upLip_ctrl_Y)) * strengthY, 'name_upLip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )
        
        # 이빨은 입술보단 좌우로 덜 움직이니 x좌표 움직임에 0.5곱함.
        pm.move(float((float(dataarray[56]) - float(PRE_name_upLip_ctrl_X)) * strengthX) * (-0.5), (float(dataarray[55]) - float(PRE_name_upLip_ctrl_Y)) * strengthY, 'name_up_teeth_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_upLip_ctrl_Z = -float(dataarray[54])
        PRE_name_upLip_ctrl_Y = dataarray[55]
        PRE_name_upLip_ctrl_X = dataarray[56]

        ############################################################################################
        # name_up_teeth_ctrl (dataarray[54] ~ dataarray[56])
        
        #global PRE_name_upLip_ctrl_X
        #global PRE_name_upLip_ctrl_Y
        #global PRE_name_upLip_ctrl_Z
        
        # 이빨은 입술보단 좌우로 덜 움직이니 x좌표 움직임에 0.5곱함.
        #pm.move(float((float(dataarray[56]) - float(PRE_name_upLip_ctrl_X)) * strengthX) * (-0.5), (float(dataarray[55]) - float(PRE_name_upLip_ctrl_Y)) * strengthY, 'name_up_teeth_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_upLip_ctrl_Z = -float(dataarray[54])
        #PRE_name_upLip_ctrl_Y = dataarray[55]
        #PRE_name_upLip_ctrl_X = dataarray[56]
        ############################################################################################
        
        # name_l_upLip_ctrl (dataarray[57] ~ dataarray[59])
        
        global PRE_name_l_upLip_ctrl_X
        global PRE_name_l_upLip_ctrl_Y
        #global PRE_name_l_upLip_ctrl_Z
        
        pm.move(float((float(dataarray[59]) - float(PRE_name_l_upLip_ctrl_X)) * strengthX) * (-1), (float(dataarray[58]) - float(PRE_name_l_upLip_ctrl_Y)) * strengthY, 'name_l_upLip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_l_upLip_ctrl_Z = -float(dataarray[57])
        PRE_name_l_upLip_ctrl_Y = dataarray[58]
        PRE_name_l_upLip_ctrl_X = dataarray[59]
        
        # name_r_upLip_ctrl (dataarray[60] ~ dataarray[62])
        
        global PRE_name_r_upLip_ctrl_X
        global PRE_name_r_upLip_ctrl_Y
        #global PRE_name_r_upLip_ctrl_Z
        
        pm.move((float(dataarray[62]) - float(PRE_name_r_upLip_ctrl_X)) * strengthX, (float(dataarray[61]) - float(PRE_name_r_upLip_ctrl_Y)) * strengthY, 'name_r_upLip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

        #PRE_name_r_upLip_ctrl_Z = -float(dataarray[60])
        PRE_name_r_upLip_ctrl_Y = dataarray[61]
        PRE_name_r_upLip_ctrl_X = dataarray[62]
        

def portData(arg):
    """
    Read the 'serial' data passed in from the commandPort
    """
    global dataarray

    dataarray=[]
        # 이차 배열의 첫 배열 dataarray.
    recVal = str(arg)
        # 받은 arg 스트링을 recVal에 넣음.
    strArray = recVal.split(",")
        # recVal 스트링을 , 경계로 쪼갬.
    for i in range(0,18):
        # 63가지의 데이터가 들어옴.
        dataarray.append(strArray[i])
        #print(strArray[i])
            # dataarray에 strArray[i] 첨부.
        #print(strArray[i])
    #createTimer(0.03, deformface)
    deformface()


def deactivateCommandPort(host, port):
    path = host + ":" + port
    active = cmds.commandPort(path, q=True)
    if active:
        cmds.commandPort(name=path, cl=True)
    else:
        print("%s is was not active" % path)


```

## Source file: src/Perseus_Marker.py

{
  "path": "src/Perseus_Marker.py",
  "blob_sha": "7b0d9dbe01b116e63221a8680803d6654ce4ff13",
  "sha256": "266fd565e9e3a78c83da8319248efefa605259ac4471034d14edf0c0feaf6613",
  "lines": 738,
  "bytes": 30326,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 35,
    "jaw": 34,
    "teeth": 11,
    "deform": 3,
    "rig": 3,
    "face": 3,
    "lip": 159
  },
  "url": "https://github.com/hamin7/Marker_based_Facial_Mocap/blob/15f39ee68a77a3fc686f120b6a21e60147afe2ad/src/Perseus_Marker.py",
  "function_signature_candidates": [
    {
      "line": 31,
      "text": "def edit_cell(row, column, value):"
    },
    {
      "line": 35,
      "text": "def startrealtimeexp(*args):"
    },
    {
      "line": 54,
      "text": "def expTrackerWindow():"
    },
    {
      "line": 193,
      "text": "def deformface():"
    },
    {
      "line": 710,
      "text": "def portData(arg):"
    },
    {
      "line": 732,
      "text": "def deactivateCommandPort(host, port):"
    }
  ]
}

```text

import maya.cmds as cmds
import os
import random
import pymel.core as pm
import socket
import sys
import maya.mel as mm
import csv

recstart = 0
selectedBlend = ""
dataarray = []
gnumcurrenttime = 1

"""
menulabelarr = ['Brow Left UP','Brow Left Down','Brow Right UP','Brow Right Down','Brow Centering','Brow outer left down','Brow outer right down','Eye Close Left','Eye Close Right','Mouse Open',
                'Mouse Left Smile','Mouse Right Smile','Mouse Left Spread','Mouse Right Spread','Mouse Left Frawn','Mouse Right Frawn','Mouse Left Centering','Mouse Right Centering','Cheek Left UP',
                'Cheek Right UP']
"""

# Our mel global proc.
melproc = """
global proc portData(string $arg){
    python(("portData(\\"" + $arg + "\\")"));
}
"""

mm.eval(melproc)


def edit_cell(row, column, value):
    return 1


def startrealtimeexp(*args):
    global recstart

    # numrow = cmds.scriptTable('scrtable', query=True, rows=True)
    if len(selectedBlend) >= 0:
        if recstart == 0:
            recstart = 1
            cmds.button('realtimecomm', edit=True, label='Stop Real Time Expression')
            # Start Comm
            cmds.commandPort(name="127.0.0.1:7777", echoOutput=False, noreturn=False, prefix="portData",
                             returnNumCommands=True)
            cmds.commandPort(name=":7777", echoOutput=False, noreturn=False, prefix="portData", returnNumCommands=True)
        else:
            recstart = 0
            cmds.button('realtimecomm', edit=True, label='Start Real Time Expression')
            # Stop Comm
            deactivateCommandPort('127.0.0.1', '7777')


def expTrackerWindow():
    if cmds.window('expTrackerWindow', exists=True):
        cmds.deleteUI('expTrackerWindow')
    # window def
    cmds.window('expTrackerWindow', widthHeight=(900, 400), title='Perseus_Boy_Facial_Mocap-conelab',
                minimizeButton=False, maximizeButton=False, resizeToFitChildren=True, sizeable=True)
    # cmds.rowColumnLayout(numberOfColumns=3,columnWidth=[(1,300),(2,300),(3,300)],backgroundColor=[200,200,0])
    cmds.columnLayout('temp3', width=900)
    cmds.rowColumnLayout(numberOfColumns=5, columnWidth=[(1, 200), (2, 50), (3, 200), (4, 150), (5, 300)])
    cmds.text(label='')
    cmds.text(label='')
    cmds.text(label='')
    cmds.text(label='')
    cmds.text(label='')
    # cmds.button( label = 'Save preset', command = savepresetfile,backgroundColor=[0.3412,0.8196,0.7882] )
    cmds.text(label='')
    # cmds.button( label = 'Load preset', command = loadpresetfile,backgroundColor=[0.3412,0.8196,0.7882] )
    cmds.text(label='')
    cmds.button('realtimecomm', label='Start Real Time Expression', command=startrealtimeexp,
                backgroundColor=[0.9294, 0.3294, 0.5216])
    # cmds.button(label='close', command=('cmds.deleteUI(\"' + window + '\", window=True)'), backgroundColor=[0.9294,0.3294,0.5216])
    cmds.text(label='')
    cmds.text(label='')
    cmds.text(label='')
    cmds.text(label='')
    cmds.text(label='')
    cmds.showWindow('expTrackerWindow')


expTrackerWindow()

# 0 ~ 2
PRE_name_Nose_ctrl_Z = 0
PRE_name_Nose_ctrl_Y = 0
PRE_name_Nose_ctrl_X = 0

# 3 ~ 5
# PRE_name_downLip_ctrl_Z = 0
PRE_name_downLip_ctrl_Y = 0
PRE_name_downLip_ctrl_X = 0

# 6 ~ 8
# PRE_name_l_downLip_ctrl_Z = 0
PRE_name_l_downLip_ctrl_Y = 0
PRE_name_l_downLip_ctrl_X = 0

# 9 ~ 11
# PRE_name_r_downLip_ctrl_Z = 0
PRE_name_r_downLip_ctrl_Y = 0
PRE_name_r_downLip_ctrl_X = 0

# 12 ~ 14
# PRE_name_l_up_cheek_ctrl_Z = 0
PRE_name_l_up_cheek_ctrl_Y = 0
PRE_name_l_up_cheek_ctrl_X = 0

# 15 ~ 17
# PRE_name_l_cheek_ctrl_Z = 0
PRE_name_l_cheek_ctrl_Y = 0
PRE_name_l_cheek_ctrl_X = 0

# 18 ~ 20
# PRE_name_l_Nose_ctrl_Z = 0
PRE_name_l_Nose_ctrl_Y = 0
PRE_name_l_Nose_ctrl_X = 0

# 21 ~ 23
# PRE_name_l_Lip_ctrl_Z = 0
PRE_name_l_Lip_ctrl_Y = 0
PRE_name_l_Lip_ctrl_X = 0

# 24 ~ 26
# PRE_name_l_jaw_cheek_ctrl_Z = 0
PRE_name_l_jaw_cheek_ctrl_Y = 0
PRE_name_l_jaw_cheek_ctrl_X = 0

# 27 ~ 29
# PRE_name_l_nose_cheek_ctrl_Z = 0
# PRE_name_l_nose_cheek_ctrl_Y = 0
# PRE_name_l_nose_cheek_ctrl_X = 0

# 30 ~ 32
# PRE_name_r_up_cheek_ctrl_Z = 0
PRE_name_r_up_cheek_ctrl_Y = 0
PRE_name_r_up_cheek_ctrl_X = 0

# 33 ~ 35
# PRE_name_r_cheek_ctrl_Z = 0
PRE_name_r_cheek_ctrl_Y = 0
PRE_name_r_cheek_ctrl_X = 0

# 36 ~ 38
# PRE_name_r_Nose_ctrl_Z = 0
PRE_name_r_Nose_ctrl_Y = 0
PRE_name_r_Nose_ctrl_X = 0

# 39 ~ 41
# PRE_name_r_Lip_ctrl_Z = 0
PRE_name_r_Lip_ctrl_Y = 0
PRE_name_r_Lip_ctrl_X = 0

# 42 ~ 44
# PRE_name_r_jaw_cheek_ctrl_Z = 0
PRE_name_r_jaw_cheek_ctrl_Y = 0
PRE_name_r_jaw_cheek_ctrl_X = 0

# 45 ~ 47
# PRE_name_r_nose_cheek_ctrl_Z = 0
# PRE_name_r_nose_cheek_ctrl_Y = 0
# PRE_name_r_nose_cheek_ctrl_X = 0

# 48 ~ 50
# PRE_name_l_down_eye_border_ctrl_Z = 0
# PRE_name_l_down_eye_border_ctrl_Y = 0
# PRE_name_l_down_eye_border_ctrl_X = 0

# 51 ~ 53
# PRE_name_r_down_eye_border_ctrl_Z = 0
# PRE_name_r_down_eye_border_ctrl_Y = 0
# PRE_name_r_down_eye_border_ctrl_X = 0

# 54 ~ 56
# PRE_name_upLip_ctrl_Z = 0
PRE_name_upLip_ctrl_Y = 0
PRE_name_upLip_ctrl_X = 0

# 57 ~ 59
# PRE_name_l_upLip_ctrl_Z = 0
PRE_name_l_upLip_ctrl_Y = 0
PRE_name_l_upLip_ctrl_X = 0

# 60 ~ 62
# PRE_name_r_upLip_ctrl_Z = 0
PRE_name_r_upLip_ctrl_Y = 0
PRE_name_r_upLip_ctrl_X = 0

isInitialized = False


def deformface():
    global dataarray
    if len(dataarray) < 0:
        # dataarray의 인자의 첫 세개는 목뼈의 x,y,z좌표 이므로 이것이 없다면 facial_mocap의 의미 없으므로 0을 리턴.
        return 0

    # all_rows = cmds.scriptTable('scrtable', query=True, rows=True)
    global recstart
    global gnumcurrenttime

    global strengthX
    global strengthY
    # global strengthZ

    strengthX = 75
    strengthY = 45
    # strengthZ = 50

    global isInitialized

    global PRE_name_Nose_ctrl_X
    global PRE_name_Nose_ctrl_Y
    # global PRE_name_Nose_ctrl_Z

    global PRE_name_downLip_ctrl_X
    global PRE_name_downLip_ctrl_Y
    # global PRE_name_downLip_ctrl_Z

    global PRE_name_l_downLip_ctrl_X
    global PRE_name_l_downLip_ctrl_Y
    # global PRE_name_l_downLip_ctrl_Z

    global PRE_name_r_downLip_ctrl_X
    global PRE_name_r_downLip_ctrl_Y
    # global PRE_name_r_downLip_ctrl_Z

    global PRE_name_l_up_cheek_ctrl_X
    global PRE_name_l_up_cheek_ctrl_Y
    # global PRE_name_l_up_cheek_ctrl_Z

    global PRE_name_l_up_cheek_ctrl_X
    global PRE_name_l_up_cheek_ctrl_Y
    # global PRE_name_l_up_cheek_ctrl_Z

    global PRE_name_l_cheek_ctrl_X
    global PRE_name_l_cheek_ctrl_Y
    # global PRE_name_l_cheek_ctrl_Z

    global PRE_name_l_Nose_ctrl_X
    global PRE_name_l_Nose_ctrl_Y
    # global PRE_name_l_Nose_ctrl_Z

    global PRE_name_l_Lip_ctrl_X
    global PRE_name_l_Lip_ctrl_Y
    # global PRE_name_l_Lip_ctrl_Z

    global PRE_name_l_Lip_ctrl_X
    global PRE_name_l_Lip_ctrl_Y
    # global PRE_name_l_Lip_ctrl_Z

    global PRE_name_l_jaw_cheek_ctrl_X
    global PRE_name_l_jaw_cheek_ctrl_Y
    # global PRE_name_l_jaw_cheek_ctrl_Z

    # global PRE_name_l_nose_cheek_ctrl_X
    # global PRE_name_l_nose_cheek_ctrl_Y
    # global PRE_name_l_nose_cheek_ctrl_Z

    global PRE_name_r_up_cheek_ctrl_X
    global PRE_name_r_up_cheek_ctrl_Y
    # global PRE_name_r_up_cheek_ctrl_Z

    global PRE_name_r_cheek_ctrl_X
    global PRE_name_r_cheek_ctrl_Y
    # global PRE_name_r_cheek_ctrl_Z

    global PRE_name_r_Nose_ctrl_X
    global PRE_name_r_Nose_ctrl_Y
    # global PRE_name_r_Nose_ctrl_Z

    global PRE_name_r_Lip_ctrl_X
    global PRE_name_r_Lip_ctrl_Y
    # global PRE_name_r_Lip_ctrl_Z
    global PRE_name_r_jaw_cheek_ctrl_X
    global PRE_name_r_jaw_cheek_ctrl_Y
    # global PRE_name_r_jaw_cheek_ctrl_Z

    # global PRE_name_r_nose_cheek_ctrl_X
    # global PRE_name_r_nose_cheek_ctrl_Y
    # global PRE_name_r_nose_cheek_ctrl_Z

    # global PRE_name_l_down_eye_border_ctrl_X
    # global PRE_name_l_down_eye_border_ctrl_Y
    # global PRE_name_l_down_eye_border_ctrl_Z

    # global PRE_name_r_down_eye_border_ctrl_X
    # global PRE_name_r_down_eye_border_ctrl_Y
    # global PRE_name_r_down_eye_border_ctrl_Z

    global PRE_name_upLip_ctrl_X
    global PRE_name_upLip_ctrl_Y
    # global PRE_name_upLip_ctrl_Z

    global PRE_name_l_upLip_ctrl_X
    global PRE_name_l_upLip_ctrl_Y
    # global PRE_name_l_upLip_ctrl_Z

    global PRE_name_r_upLip_ctrl_X
    global PRE_name_r_upLip_ctrl_Y
    # global PRE_name_r_upLip_ctrl_Z

    all_rows = 1
    if all_rows > 0 and recstart == 1:

        if isInitialized:
            # name_Nose_ctrl (dataarray[0] ~ dataarray[2])
            pm.move((float(dataarray[2]) - float(PRE_name_Nose_ctrl_X)) * strengthX,
                    (float(dataarray[1]) - float(PRE_name_Nose_ctrl_Y)) * strengthY, 'name_Nose_ctrl', relative=True,
                    objectSpace=True, worldSpaceDistance=True)
            # PRE_name_Nose_ctrl_Z = -float(dataarray[0])
            PRE_name_Nose_ctrl_Y = dataarray[1]
            PRE_name_Nose_ctrl_X = dataarray[2]
            # name_downLip_ctrl (dataarray[3] ~ dataarray[5])
            # name_down_teeth_ctrl (dataarray[3] ~ dataarray[5])
            # name_jaw_ctrl (dataarray[3] ~ dataarray[5])

            pm.move(float((float(dataarray[5]) - float(PRE_name_downLip_ctrl_X)) * strengthX) * (-1),
                    float((float(dataarray[4]) - float(PRE_name_downLip_ctrl_Y)) * strengthY) * (-1),
                    'name_downLip_ctrl',
                    relative=True, objectSpace=True, worldSpaceDistance=True)
            # 치아는 입술보단 덜 움직이므로 x 좌표 움직임에 0.8 곱함.
            pm.move(float((float(dataarray[5]) - float(PRE_name_downLip_ctrl_X)) * strengthX) * (-0.5),
                    float((float(dataarray[4]) - float(PRE_name_downLip_ctrl_Y)) * strengthY) * (0.8),
                    'name_down_teeth_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)
            # 턱도 아랫입술과 같이 움직임
            pm.move(float((float(dataarray[5]) - float(PRE_name_downLip_ctrl_X)) * strengthX) * (-0.5),
                    float((float(dataarray[4]) - float(PRE_name_downLip_ctrl_Y)) * strengthY) * (0.8), 'name_jaw_ctrl',
                    relative=True, objectSpace=True, worldSpaceDistance=True)

            # PRE_name_downLip_ctrl_Z = -float(dataarray[3])
            PRE_name_downLip_ctrl_Y = dataarray[4]
            PRE_name_downLip_ctrl_X = dataarray[5]

            ############################################################################################
            # name_down_teeth_ctrl (dataarray[3] ~ dataarray[5])

            # global PRE_name_downLip_ctrl_X
            # global PRE_name_downLip_ctrl_Y
            # global PRE_name_downLip_ctrl_Z

            # 치아는 입술보단 덜 움직이므로 x 좌표 움직임에 0.7 곱함.
            # pm.move(float((float(dataarray[5]) - float(PRE_name_downLip_ctrl_X)) * strengthX) * (-0.7), float((float(dataarray[4]) - float(PRE_name_downLip_ctrl_Y)) * strengthY) * (-1), 'name_down_teeth_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

            # PRE_name_downLip_ctrl_Z = -float(dataarray[3])
            # PRE_name_downLip_ctrl_Y = dataarray[4]
            # PRE_name_downLip_ctrl_X = dataarray[5]
            ############################################################################################

            # name_l_downLip_ctrl (dataarray[6] ~ dataarray[8])

            pm.move(float((float(dataarray[8]) - float(PRE_name_l_downLip_ctrl_X)) * strengthX) * (-1),
                    float((float(dataarray[7]) - float(PRE_name_l_downLip_ctrl_Y)) * strengthY) * (-1),
                    'name_l_downLip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            # PRE_name_l_downLip_ctrl_Z = -float(dataarray[6])
            PRE_name_l_downLip_ctrl_Y = dataarray[7]
            PRE_name_l_downLip_ctrl_X = dataarray[8]

            # name_r_downLip_ctrl (dataarray[9] ~ dataarray[11])

            pm.move((float(dataarray[11]) - float(PRE_name_r_downLip_ctrl_X)) * strengthX,
                    float((float(dataarray[10]) - float(PRE_name_r_downLip_ctrl_Y)) * strengthY) * (-1),
                    'name_r_downLip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            # PRE_name_r_downLip_ctrl_Z = -float(dataarray[9])
            PRE_name_r_downLip_ctrl_Y = dataarray[10]
            PRE_name_r_downLip_ctrl_X = dataarray[11]

            # name_l_up_cheek_ctrl (dataarray[12] ~ dataarray[14])

            pm.move((float(dataarray[14]) - float(PRE_name_l_up_cheek_ctrl_X)) * strengthX,
                    (float(dataarray[13]) - float(PRE_name_l_up_cheek_ctrl_Y)) * strengthY, 'name_l_up_cheek_ctrl',
                    relative=True, objectSpace=True, worldSpaceDistance=True)

            # PRE_name_l_up_cheek_ctrl_Z = -float(dataarray[12])
            PRE_name_l_up_cheek_ctrl_Y = dataarray[13]
            PRE_name_l_up_cheek_ctrl_X = dataarray[14]

            # name_l_cheek_ctrl (dataarray[15] ~ dataarray[17])

            pm.move((float(dataarray[17]) - float(PRE_name_l_cheek_ctrl_X)) * strengthX,
                    (float(dataarray[16]) - float(PRE_name_l_cheek_ctrl_Y)) * strengthY, 'name_l_cheek_ctrl',
                    relative=True,
                    objectSpace=True, worldSpaceDistance=True)

            # PRE_name_l_cheek_ctrl_Z = -float(dataarray[15])
            PRE_name_l_cheek_ctrl_Y = dataarray[16]
            PRE_name_l_cheek_ctrl_X = dataarray[17]

            # name_l_Nose_ctrl (dataarray[18] ~ dataarray[20])

            pm.move((float(dataarray[20]) - float(PRE_name_l_Nose_ctrl_X)) * strengthX,
                    (float(dataarray[19]) - float(PRE_name_l_Nose_ctrl_Y)) * strengthY, 'name_l_Nose_ctrl',
                    relative=True,
                    objectSpace=True, worldSpaceDistance=True)

            # PRE_name_l_Nose_ctrl_Z = -float(dataarray[18])
            PRE_name_l_Nose_ctrl_Y = dataarray[19]
            PRE_name_l_Nose_ctrl_X = dataarray[20]

            # name_l_Lip_ctrl (dataarray[21] ~ dataarray[23])

            pm.move(float((float(dataarray[23]) - float(PRE_name_l_Lip_ctrl_X)) * strengthX) * (-1),
                    (float(dataarray[22]) - float(PRE_name_l_Lip_ctrl_Y)) * strengthY, 'name_l_Lip_ctrl', relative=True,
                    objectSpace=True, worldSpaceDistance=True)

            # PRE_name_l_Lip_ctrl_Z = -float(dataarray[21])
            PRE_name_l_Lip_ctrl_Y = dataarray[22]
            PRE_name_l_Lip_ctrl_X = dataarray[23]

            # name_l_jaw_cheek_ctrl (dataarray[24] ~ dataarray[26])

            pm.move((float(dataarray[26]) - float(PRE_name_l_jaw_cheek_ctrl_X)) * strengthX,
                    (float(dataarray[25]) - float(PRE_name_l_jaw_cheek_ctrl_Y)) * strengthY, 'name_l_jaw_cheek_ctrl',
                    relative=True, objectSpace=True, worldSpaceDistance=True)

            # PRE_name_l_jaw_cheek_ctrl_Z = -float(dataarray[24])
            PRE_name_l_jaw_cheek_ctrl_Y = dataarray[25]
            PRE_name_l_jaw_cheek_ctrl_X = dataarray[26]

            # name_l_nose_cheek_ctrl (dataarray[27] ~ dataarray[29])

            # pm.move((float(dataarray[29]) - float(PRE_name_l_nose_cheek_ctrl_X)) * strengthX,
            # (float(dataarray[28]) - float(PRE_name_l_nose_cheek_ctrl_Y)) * strengthY, 'name_l_nose_cheek_ctrl',
            # relative=True, objectSpace=True, worldSpaceDistance=True)

            # PRE_name_l_nose_cheek_ctrl_Z = -float(dataarray[27])
            # PRE_name_l_nose_cheek_ctrl_Y = dataarray[28]
            # PRE_name_l_nose_cheek_ctrl_X = dataarray[29]

            # name_r_up_cheek_ctrl (dataarray[30] ~ dataarray[32])

            pm.move((float(dataarray[29]) - float(PRE_name_r_up_cheek_ctrl_X)) * strengthX,
                    (float(dataarray[28]) - float(PRE_name_r_up_cheek_ctrl_Y)) * strengthY, 'name_r_up_cheek_ctrl',
                    relative=True, objectSpace=True, worldSpaceDistance=True)

            # PRE_name_r_up_cheek_ctrl_Z = -float(dataarray[30])
            PRE_name_r_up_cheek_ctrl_Y = dataarray[28]
            PRE_name_r_up_cheek_ctrl_X = dataarray[29]

            # name_r_cheek_ctrl (dataarray[33] ~ dataarray[35])

            pm.move((float(dataarray[32]) - float(PRE_name_r_cheek_ctrl_X)) * strengthX,
                    (float(dataarray[31]) - float(PRE_name_r_cheek_ctrl_Y)) * strengthY, 'name_r_cheek_ctrl',
                    relative=True,
                    objectSpace=True, worldSpaceDistance=True)

            # PRE_name_r_cheek_ctrl_Z = -float(dataarray[33])
            PRE_name_r_cheek_ctrl_Y = dataarray[31]
            PRE_name_r_cheek_ctrl_X = dataarray[32]

            # name_r_Nose_ctrl (dataarray[36] ~ dataarray[38])

            pm.move((float(dataarray[35]) - float(PRE_name_r_Nose_ctrl_X)) * strengthX,
                    (float(dataarray[34]) - float(PRE_name_r_Nose_ctrl_Y)) * strengthY, 'name_r_Nose_ctrl',
                    relative=True,
                    objectSpace=True, worldSpaceDistance=True)

            # PRE_name_r_Nose_ctrl_Z = -float(dataarray[36])
            PRE_name_r_Nose_ctrl_Y = dataarray[34]
            PRE_name_r_Nose_ctrl_X = dataarray[35]

            # name_r_Lip_ctrl (dataarray[39] ~ dataarray[41])

            pm.move((float(dataarray[38]) - float(PRE_name_r_Lip_ctrl_X)) * strengthX,
                    (float(dataarray[37]) - float(PRE_name_r_Lip_ctrl_Y)) * strengthY, 'name_r_Lip_ctrl', relative=True,
                    objectSpace=True, worldSpaceDistance=True)

            # PRE_name_r_Lip_ctrl_Z = -float(dataarray[39])
            PRE_name_r_Lip_ctrl_Y = dataarray[37]
            PRE_name_r_Lip_ctrl_X = dataarray[38]

            # name_r_jaw_cheek_ctrl (dataarray[42] ~ dataarray[44])

            pm.move((float(dataarray[41]) - float(PRE_name_r_jaw_cheek_ctrl_X)) * strengthX,
                    (float(dataarray[40]) - float(PRE_name_r_jaw_cheek_ctrl_Y)) * strengthY, 'name_r_jaw_cheek_ctrl',
                    relative=True, objectSpace=True, worldSpaceDistance=True)

            # PRE_name_r_jaw_cheek_ctrl_Z = -float(dataarray[42])
            PRE_name_r_jaw_cheek_ctrl_Y = dataarray[40]
            PRE_name_r_jaw_cheek_ctrl_X = dataarray[41]

            # name_r_nose_cheek_ctrl (dataarray[45] ~ dataarray[47])

            # pm.move((float(dataarray[47]) - float(PRE_name_r_nose_cheek_ctrl_X)) * strengthX,
            # (float(dataarray[46]) - float(PRE_name_r_nose_cheek_ctrl_Y)) * strengthY, 'name_r_nose_cheek_ctrl',
            # relative=True, objectSpace=True, worldSpaceDistance=True)

            # PRE_name_r_nose_cheek_ctrl_Z = -float(dataarray[45])
            # PRE_name_r_nose_cheek_ctrl_Y = dataarray[46]
            # PRE_name_r_nose_cheek_ctrl_X = dataarray[47]

            # name_l_down_eye_border_ctrl (dataarray[48] ~ dataarray[50])

            # pm.move((float(dataarray[50]) - float(PRE_name_l_down_eye_border_ctrl_X)) * strengthX,
            # (float(dataarray[49]) - float(PRE_name_l_down_eye_border_ctrl_Y)) * strengthY,
            # 'name_l_down_eye_border_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            # PRE_name_l_down_eye_border_ctrl_Z = -float(dataarray[48])
            # PRE_name_l_down_eye_border_ctrl_Y = dataarray[49]
            # PRE_name_l_down_eye_border_ctrl_X = dataarray[50]

            # name_r_down_eye_border_ctrl (dataarray[51] ~ dataarray[53])

            # pm.move((float(dataarray[53]) - float(PRE_name_r_down_eye_border_ctrl_X)) * strengthX,
            # (float(dataarray[52]) - float(PRE_name_r_down_eye_border_ctrl_Y)) * strengthY,
            # 'name_r_down_eye_border_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            # PRE_name_r_down_eye_border_ctrl_Z = -float(dataarray[51])
            # PRE_name_r_down_eye_border_ctrl_Y = dataarray[52]
            # PRE_name_r_down_eye_border_ctrl_X = dataarray[53]

            # name_upLip_ctrl (dataarray[54] ~ dataarray[56])
            # name_up_teeth_ctrl (dataarray[54] ~ dataarray[56])

            pm.move(float((float(dataarray[44]) - float(PRE_name_upLip_ctrl_X)) * strengthX) * (-1),
                    (float(dataarray[43]) - float(PRE_name_upLip_ctrl_Y)) * strengthY, 'name_upLip_ctrl', relative=True,
                    objectSpace=True, worldSpaceDistance=True)

            # 이빨은 입술보단 좌우로 덜 움직이니 x좌표 움직임에 0.5곱함.
            pm.move(float((float(dataarray[44]) - float(PRE_name_upLip_ctrl_X)) * strengthX) * (-0.5),
                    (float(dataarray[43]) - float(PRE_name_upLip_ctrl_Y)) * strengthY, 'name_up_teeth_ctrl',
                    relative=True,
                    objectSpace=True, worldSpaceDistance=True)

            # PRE_name_upLip_ctrl_Z = -float(dataarray[54])
            PRE_name_upLip_ctrl_Y = dataarray[43]
            PRE_name_upLip_ctrl_X = dataarray[44]

            ############################################################################################
            # name_up_teeth_ctrl (dataarray[54] ~ dataarray[56])

            # global PRE_name_upLip_ctrl_X
            # global PRE_name_upLip_ctrl_Y
            # global PRE_name_upLip_ctrl_Z

            # 이빨은 입술보단 좌우로 덜 움직이니 x좌표 움직임에 0.5곱함.
            # pm.move(float((float(dataarray[56]) - float(PRE_name_upLip_ctrl_X)) * strengthX) * (-0.5), (float(dataarray[55]) - float(PRE_name_upLip_ctrl_Y)) * strengthY, 'name_up_teeth_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

            # PRE_name_upLip_ctrl_Z = -float(dataarray[54])
            # PRE_name_upLip_ctrl_Y = dataarray[55]
            # PRE_name_upLip_ctrl_X = dataarray[56]
            ############################################################################################

            # name_l_upLip_ctrl (dataarray[57] ~ dataarray[59])

            pm.move(float((float(dataarray[47]) - float(PRE_name_l_upLip_ctrl_X)) * strengthX) * (-1),
                    (float(dataarray[46]) - float(PRE_name_l_upLip_ctrl_Y)) * strengthY, 'name_l_upLip_ctrl',
                    relative=True,
                    objectSpace=True, worldSpaceDistance=True)

            # PRE_name_l_upLip_ctrl_Z = -float(dataarray[57])
            PRE_name_l_upLip_ctrl_Y = dataarray[46]
            PRE_name_l_upLip_ctrl_X = dataarray[47]

            # name_r_upLip_ctrl (dataarray[60] ~ dataarray[62])

            pm.move((float(dataarray[50]) - float(PRE_name_r_upLip_ctrl_X)) * strengthX,
                    (float(dataarray[49]) - float(PRE_name_r_upLip_ctrl_Y)) * strengthY, 'name_r_upLip_ctrl',
                    relative=True,
                    objectSpace=True, worldSpaceDistance=True)

            # PRE_name_r_upLip_ctrl_Z = -float(dataarray[60])
            PRE_name_r_upLip_ctrl_Y = dataarray[49]
            PRE_name_r_upLip_ctrl_X = dataarray[50]
        else:
            # name_Nose_ctrl (dataarray[0] ~ dataarray[2])

            # PRE_name_Nose_ctrl_Z = -float(dataarray[0])
            PRE_name_Nose_ctrl_Y = dataarray[1]
            PRE_name_Nose_ctrl_X = dataarray[2]

            # PRE_name_downLip_ctrl_Z = -float(dataarray[3])
            PRE_name_downLip_ctrl_Y = dataarray[4]
            PRE_name_downLip_ctrl_X = dataarray[5]

            # PRE_name_l_downLip_ctrl_Z = -float(dataarray[6])
            PRE_name_l_downLip_ctrl_Y = dataarray[7]
            PRE_name_l_downLip_ctrl_X = dataarray[8]

            # name_r_downLip_ctrl (dataarray[9] ~ dataarray[11])

            # PRE_name_r_downLip_ctrl_Z = -float(dataarray[9])
            PRE_name_r_downLip_ctrl_Y = dataarray[10]
            PRE_name_r_downLip_ctrl_X = dataarray[11]

            # name_l_up_cheek_ctrl (dataarray[12] ~ dataarray[14])

            # PRE_name_l_up_cheek_ctrl_Z = -float(dataarray[12])
            PRE_name_l_up_cheek_ctrl_Y = dataarray[13]
            PRE_name_l_up_cheek_ctrl_X = dataarray[14]

            # name_l_cheek_ctrl (dataarray[15] ~ dataarray[17])

            # PRE_name_l_cheek_ctrl_Z = -float(dataarray[15])
            PRE_name_l_cheek_ctrl_Y = dataarray[16]
            PRE_name_l_cheek_ctrl_X = dataarray[17]

            # name_l_Nose_ctrl (dataarray[18] ~ dataarray[20])

            # PRE_name_l_Nose_ctrl_Z = -float(dataarray[18])
            PRE_name_l_Nose_ctrl_Y = dataarray[19]
            PRE_name_l_Nose_ctrl_X = dataarray[20]

            # name_l_Lip_ctrl (dataarray[21] ~ dataarray[23])

            # PRE_name_l_Lip_ctrl_Z = -float(dataarray[21])
            PRE_name_l_Lip_ctrl_Y = dataarray[22]
            PRE_name_l_Lip_ctrl_X = dataarray[23]

            # name_l_jaw_cheek_ctrl (dataarray[24] ~ dataarray[26])

            # PRE_name_l_jaw_cheek_ctrl_Z = -float(dataarray[24])
            PRE_name_l_jaw_cheek_ctrl_Y = dataarray[25]
            PRE_name_l_jaw_cheek_ctrl_X = dataarray[26]

            # name_l_nose_cheek_ctrl (dataarray[27] ~ dataarray[29])

            # PRE_name_l_nose_cheek_ctrl_Z = -float(dataarray[27])
            # PRE_name_l_nose_cheek_ctrl_Y = dataarray[28]
            # PRE_name_l_nose_cheek_ctrl_X = dataarray[29]

            # name_r_up_cheek_ctrl (dataarray[30] ~ dataarray[32])

            # PRE_name_r_up_cheek_ctrl_Z = -float(dataarray[30])
            PRE_name_r_up_cheek_ctrl_Y = dataarray[28]
            PRE_name_r_up_cheek_ctrl_X = dataarray[29]

            # name_r_cheek_ctrl (dataarray[33] ~ dataarray[35])

            # PRE_name_r_cheek_ctrl_Z = -float(dataarray[33])
            PRE_name_r_cheek_ctrl_Y = dataarray[31]
            PRE_name_r_cheek_ctrl_X = dataarray[32]

            # name_r_Nose_ctrl (dataarray[36] ~ dataarray[38])

            # PRE_name_r_Nose_ctrl_Z = -float(dataarray[36])
            PRE_name_r_Nose_ctrl_Y = dataarray[34]
            PRE_name_r_Nose_ctrl_X = dataarray[35]

            # name_r_Lip_ctrl (dataarray[39] ~ dataarray[41])

            # PRE_name_r_Lip_ctrl_Z = -float(dataarray[39])
            PRE_name_r_Lip_ctrl_Y = dataarray[37]
            PRE_name_r_Lip_ctrl_X = dataarray[38]

            # name_r_jaw_cheek_ctrl (dataarray[42] ~ dataarray[44])

            # PRE_name_r_jaw_cheek_ctrl_Z = -float(dataarray[42])
            PRE_name_r_jaw_cheek_ctrl_Y = dataarray[40]
            PRE_name_r_jaw_cheek_ctrl_X = dataarray[41]

            # name_r_nose_cheek_ctrl (dataarray[45] ~ dataarray[47])

            # PRE_name_r_nose_cheek_ctrl_Z = -float(dataarray[45])
            # PRE_name_r_nose_cheek_ctrl_Y = dataarray[46]
            # PRE_name_r_nose_cheek_ctrl_X = dataarray[47]

            # name_l_down_eye_border_ctrl (dataarray[48] ~ dataarray[50])

            # PRE_name_l_down_eye_border_ctrl_Z = -float(dataarray[48])
            # PRE_name_l_down_eye_border_ctrl_Y = dataarray[49]
            # PRE_name_l_down_eye_border_ctrl_X = dataarray[50]

            # name_r_down_eye_border_ctrl (dataarray[51] ~ dataarray[53])

            # PRE_name_r_down_eye_border_ctrl_Z = -float(dataarray[51])
            # PRE_name_r_down_eye_border_ctrl_Y = dataarray[52]
            # PRE_name_r_down_eye_border_ctrl_X = dataarray[53]

            # name_upLip_ctrl (dataarray[54] ~ dataarray[56])
            # name_up_teeth_ctrl (dataarray[54] ~ dataarray[56])

            # PRE_name_upLip_ctrl_Z = -float(dataarray[54])
            PRE_name_upLip_ctrl_Y = dataarray[43]
            PRE_name_upLip_ctrl_X = dataarray[44]

            ############################################################################################
            # name_up_teeth_ctrl (dataarray[54] ~ dataarray[56])

            # global PRE_name_upLip_ctrl_X
            # global PRE_name_upLip_ctrl_Y
            # global PRE_name_upLip_ctrl_Z

            # 이빨은 입술보단 좌우로 덜 움직이니 x좌표 움직임에 0.5곱함.
            # pm.move(float((float(dataarray[56]) - float(PRE_name_upLip_ctrl_X)) * strengthX) * (-0.5), (float(dataarray[55]) - float(PRE_name_upLip_ctrl_Y)) * strengthY, 'name_up_teeth_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

            # PRE_name_upLip_ctrl_Z = -float(dataarray[54])
            # PRE_name_upLip_ctrl_Y = dataarray[55]
            # PRE_name_upLip_ctrl_X = dataarray[56]
            ############################################################################################

            # name_l_upLip_ctrl (dataarray[57] ~ dataarray[59])

            # PRE_name_l_upLip_ctrl_Z = -float(dataarray[57])
            PRE_name_l_upLip_ctrl_Y = dataarray[46]
            PRE_name_l_upLip_ctrl_X = dataarray[47]

            # name_r_upLip_ctrl (dataarray[60] ~ dataarray[62])

            # PRE_name_r_upLip_ctrl_Z = -float(dataarray[60])
            PRE_name_r_upLip_ctrl_Y = dataarray[49]
            PRE_name_r_upLip_ctrl_X = dataarray[50]

            isInitialized = True;


def portData(arg):
    """
    Read the 'serial' data passed in from the commandPort
    """
    global dataarray

    dataarray = []
    # 이차 배열의 첫 배열 dataarray.
    recVal = str(arg)
    # 받은 arg 스트링을 recVal에 넣음.
    strArray = recVal.split(",")
    # recVal 스트링을 , 경계로 쪼갬.
    for i in range(0, 51):
        # 51가지의 데이터가 들어옴.
        dataarray.append(strArray[i])
        # print(strArray[i])
        # dataarray에 strArray[i] 첨부.
        # print(strArray[i])
    # createTimer(0.03, deformface)
    deformface()


def deactivateCommandPort(host, port):
    path = host + ":" + port
    active = cmds.commandPort(path, q=True)
    if active:
        cmds.commandPort(name=path, cl=True)
    else:
        print("%s is was not active" % path)


```

## Source file: src/Perseus_Marker_zAxis.py

{
  "path": "src/Perseus_Marker_zAxis.py",
  "blob_sha": "b6b79dbab1c33a1d40fda36c456aaeee59e853fe",
  "sha256": "bfb2612f15b1e0176d7ab1ab34936d33bf9dbaf70f3d45e58813957c265ff43d",
  "lines": 758,
  "bytes": 31863,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 37,
    "jaw": 38,
    "teeth": 12,
    "deform": 3,
    "rig": 3,
    "face": 3,
    "lip": 172
  },
  "url": "https://github.com/hamin7/Marker_based_Facial_Mocap/blob/15f39ee68a77a3fc686f120b6a21e60147afe2ad/src/Perseus_Marker_zAxis.py",
  "function_signature_candidates": [
    {
      "line": 31,
      "text": "def edit_cell(row, column, value):"
    },
    {
      "line": 35,
      "text": "def startrealtimeexp(*args):"
    },
    {
      "line": 54,
      "text": "def expTrackerWindow():"
    },
    {
      "line": 193,
      "text": "def deformface():"
    },
    {
      "line": 730,
      "text": "def portData(arg):"
    },
    {
      "line": 752,
      "text": "def deactivateCommandPort(host, port):"
    }
  ]
}

```text

import maya.cmds as cmds
import os
import random
import pymel.core as pm
import socket
import sys
import maya.mel as mm
import csv

recstart = 0
selectedBlend = ""
dataarray = []
gnumcurrenttime = 1

"""
menulabelarr = ['Brow Left UP','Brow Left Down','Brow Right UP','Brow Right Down','Brow Centering','Brow outer left down','Brow outer right down','Eye Close Left','Eye Close Right','Mouse Open',
                'Mouse Left Smile','Mouse Right Smile','Mouse Left Spread','Mouse Right Spread','Mouse Left Frawn','Mouse Right Frawn','Mouse Left Centering','Mouse Right Centering','Cheek Left UP',
                'Cheek Right UP']
"""

# Our mel global proc.
melproc = """
global proc portData(string $arg){
    python(("portData(\\"" + $arg + "\\")"));
}
"""

mm.eval(melproc)


def edit_cell(row, column, value):
    return 1


def startrealtimeexp(*args):
    global recstart

    # numrow = cmds.scriptTable('scrtable', query=True, rows=True)
    if len(selectedBlend) >= 0:
        if recstart == 0:
            recstart = 1
            cmds.button('realtimecomm', edit=True, label='Stop Real Time Expression')
            # Start Comm
            cmds.commandPort(name="127.0.0.1:7777", echoOutput=False, noreturn=False, prefix="portData",
                             returnNumCommands=True)
            cmds.commandPort(name=":7777", echoOutput=False, noreturn=False, prefix="portData", returnNumCommands=True)
        else:
            recstart = 0
            cmds.button('realtimecomm', edit=True, label='Start Real Time Expression')
            # Stop Comm
            deactivateCommandPort('127.0.0.1', '7777')


def expTrackerWindow():
    if cmds.window('expTrackerWindow', exists=True):
        cmds.deleteUI('expTrackerWindow')
    # window def
    cmds.window('expTrackerWindow', widthHeight=(900, 400), title='Perseus_Boy_Facial_Mocap-conelab',
                minimizeButton=False, maximizeButton=False, resizeToFitChildren=True, sizeable=True)
    # cmds.rowColumnLayout(numberOfColumns=3,columnWidth=[(1,300),(2,300),(3,300)],backgroundColor=[200,200,0])
    cmds.columnLayout('temp3', width=900)
    cmds.rowColumnLayout(numberOfColumns=5, columnWidth=[(1, 200), (2, 50), (3, 200), (4, 150), (5, 300)])
    cmds.text(label='')
    cmds.text(label='')
    cmds.text(label='')
    cmds.text(label='')
    cmds.text(label='')
    # cmds.button( label = 'Save preset', command = savepresetfile,backgroundColor=[0.3412,0.8196,0.7882] )
    cmds.text(label='')
    # cmds.button( label = 'Load preset', command = loadpresetfile,backgroundColor=[0.3412,0.8196,0.7882] )
    cmds.text(label='')
    cmds.button('realtimecomm', label='Start Real Time Expression', command=startrealtimeexp,
                backgroundColor=[0.9294, 0.3294, 0.5216])
    # cmds.button(label='close', command=('cmds.deleteUI(\"' + window + '\", window=True)'), backgroundColor=[0.9294,0.3294,0.5216])
    cmds.text(label='')
    cmds.text(label='')
    cmds.text(label='')
    cmds.text(label='')
    cmds.text(label='')
    cmds.showWindow('expTrackerWindow')


expTrackerWindow()

# 0 ~ 2
PRE_name_Nose_ctrl_Z = 0
PRE_name_Nose_ctrl_Y = 0
PRE_name_Nose_ctrl_X = 0

# 3 ~ 5
PRE_name_downLip_ctrl_Z = 0
PRE_name_downLip_ctrl_Y = 0
PRE_name_downLip_ctrl_X = 0

# 6 ~ 8
PRE_name_l_downLip_ctrl_Z = 0
PRE_name_l_downLip_ctrl_Y = 0
PRE_name_l_downLip_ctrl_X = 0

# 9 ~ 11
PRE_name_r_downLip_ctrl_Z = 0
PRE_name_r_downLip_ctrl_Y = 0
PRE_name_r_downLip_ctrl_X = 0

# 12 ~ 14
PRE_name_l_up_cheek_ctrl_Z = 0
PRE_name_l_up_cheek_ctrl_Y = 0
PRE_name_l_up_cheek_ctrl_X = 0

# 15 ~ 17
PRE_name_l_cheek_ctrl_Z = 0
PRE_name_l_cheek_ctrl_Y = 0
PRE_name_l_cheek_ctrl_X = 0

# 18 ~ 20
PRE_name_l_Nose_ctrl_Z = 0
PRE_name_l_Nose_ctrl_Y = 0
PRE_name_l_Nose_ctrl_X = 0

# 21 ~ 23
PRE_name_l_Lip_ctrl_Z = 0
PRE_name_l_Lip_ctrl_Y = 0
PRE_name_l_Lip_ctrl_X = 0

# 24 ~ 26
PRE_name_l_jaw_cheek_ctrl_Z = 0
PRE_name_l_jaw_cheek_ctrl_Y = 0
PRE_name_l_jaw_cheek_ctrl_X = 0

# 27 ~ 29
PRE_name_l_nose_cheek_ctrl_Z = 0
PRE_name_l_nose_cheek_ctrl_Y = 0
PRE_name_l_nose_cheek_ctrl_X = 0

# 30 ~ 32
PRE_name_r_up_cheek_ctrl_Z = 0
PRE_name_r_up_cheek_ctrl_Y = 0
PRE_name_r_up_cheek_ctrl_X = 0

# 33 ~ 35
PRE_name_r_cheek_ctrl_Z = 0
PRE_name_r_cheek_ctrl_Y = 0
PRE_name_r_cheek_ctrl_X = 0

# 36 ~ 38
PRE_name_r_Nose_ctrl_Z = 0
PRE_name_r_Nose_ctrl_Y = 0
PRE_name_r_Nose_ctrl_X = 0

# 39 ~ 41
PRE_name_r_Lip_ctrl_Z = 0
PRE_name_r_Lip_ctrl_Y = 0
PRE_name_r_Lip_ctrl_X = 0

# 42 ~ 44
PRE_name_r_jaw_cheek_ctrl_Z = 0
PRE_name_r_jaw_cheek_ctrl_Y = 0
PRE_name_r_jaw_cheek_ctrl_X = 0

# 45 ~ 47
PRE_name_r_nose_cheek_ctrl_Z = 0
PRE_name_r_nose_cheek_ctrl_Y = 0
PRE_name_r_nose_cheek_ctrl_X = 0

# 48 ~ 50
PRE_name_l_down_eye_border_ctrl_Z = 0
PRE_name_l_down_eye_border_ctrl_Y = 0
PRE_name_l_down_eye_border_ctrl_X = 0

# 51 ~ 53
PRE_name_r_down_eye_border_ctrl_Z = 0
PRE_name_r_down_eye_border_ctrl_Y = 0
PRE_name_r_down_eye_border_ctrl_X = 0

# 54 ~ 56
PRE_name_upLip_ctrl_Z = 0
PRE_name_upLip_ctrl_Y = 0
PRE_name_upLip_ctrl_X = 0

# 57 ~ 59
PRE_name_l_upLip_ctrl_Z = 0
PRE_name_l_upLip_ctrl_Y = 0
PRE_name_l_upLip_ctrl_X = 0

# 60 ~ 62
PRE_name_r_upLip_ctrl_Z = 0
PRE_name_r_upLip_ctrl_Y = 0
PRE_name_r_upLip_ctrl_X = 0

isInitialized = False


def deformface():
    global dataarray
    if len(dataarray) < 0:
        # dataarray의 인자의 첫 세개는 목뼈의 x,y,z좌표 이므로 이것이 없다면 facial_mocap의 의미 없으므로 0을 리턴.
        return 0

    # all_rows = cmds.scriptTable('scrtable', query=True, rows=True)
    global recstart
    global gnumcurrenttime

    global strengthX
    global strengthY
    # global strengthZ

    strengthX = 75
    strengthY = 45
    strengthZ = 50

    global isInitialized

    global PRE_name_Nose_ctrl_X
    global PRE_name_Nose_ctrl_Y
    global PRE_name_Nose_ctrl_Z

    global PRE_name_downLip_ctrl_X
    global PRE_name_downLip_ctrl_Y
    global PRE_name_downLip_ctrl_Z

    global PRE_name_l_downLip_ctrl_X
    global PRE_name_l_downLip_ctrl_Y
    global PRE_name_l_downLip_ctrl_Z

    global PRE_name_r_downLip_ctrl_X
    global PRE_name_r_downLip_ctrl_Y
    global PRE_name_r_downLip_ctrl_Z

    global PRE_name_l_up_cheek_ctrl_X
    global PRE_name_l_up_cheek_ctrl_Y
    global PRE_name_l_up_cheek_ctrl_Z

    global PRE_name_l_up_cheek_ctrl_X
    global PRE_name_l_up_cheek_ctrl_Y
    global PRE_name_l_up_cheek_ctrl_Z

    global PRE_name_l_cheek_ctrl_X
    global PRE_name_l_cheek_ctrl_Y
    global PRE_name_l_cheek_ctrl_Z

    global PRE_name_l_Nose_ctrl_X
    global PRE_name_l_Nose_ctrl_Y
    global PRE_name_l_Nose_ctrl_Z

    global PRE_name_l_Lip_ctrl_X
    global PRE_name_l_Lip_ctrl_Y
    global PRE_name_l_Lip_ctrl_Z

    global PRE_name_l_Lip_ctrl_X
    global PRE_name_l_Lip_ctrl_Y
    global PRE_name_l_Lip_ctrl_Z

    global PRE_name_l_jaw_cheek_ctrl_X
    global PRE_name_l_jaw_cheek_ctrl_Y
    global PRE_name_l_jaw_cheek_ctrl_Z

    global PRE_name_l_nose_cheek_ctrl_X
    global PRE_name_l_nose_cheek_ctrl_Y
    global PRE_name_l_nose_cheek_ctrl_Z

    global PRE_name_r_up_cheek_ctrl_X
    global PRE_name_r_up_cheek_ctrl_Y
    global PRE_name_r_up_cheek_ctrl_Z

    global PRE_name_r_cheek_ctrl_X
    global PRE_name_r_cheek_ctrl_Y
    global PRE_name_r_cheek_ctrl_Z

    global PRE_name_r_Nose_ctrl_X
    global PRE_name_r_Nose_ctrl_Y
    global PRE_name_r_Nose_ctrl_Z

    global PRE_name_r_Lip_ctrl_X
    global PRE_name_r_Lip_ctrl_Y
    global PRE_name_r_Lip_ctrl_Z

    global PRE_name_r_jaw_cheek_ctrl_X
    global PRE_name_r_jaw_cheek_ctrl_Y
    global PRE_name_r_jaw_cheek_ctrl_Z

    global PRE_name_r_nose_cheek_ctrl_X
    global PRE_name_r_nose_cheek_ctrl_Y
    global PRE_name_r_nose_cheek_ctrl_Z

    global PRE_name_l_down_eye_border_ctrl_X
    global PRE_name_l_down_eye_border_ctrl_Y
    global PRE_name_l_down_eye_border_ctrl_Z

    global PRE_name_r_down_eye_border_ctrl_X
    global PRE_name_r_down_eye_border_ctrl_Y
    global PRE_name_r_down_eye_border_ctrl_Z

    global PRE_name_upLip_ctrl_X
    global PRE_name_upLip_ctrl_Y
    global PRE_name_upLip_ctrl_Z

    global PRE_name_l_upLip_ctrl_X
    global PRE_name_l_upLip_ctrl_Y
    global PRE_name_l_upLip_ctrl_Z

    global PRE_name_r_upLip_ctrl_X
    global PRE_name_r_upLip_ctrl_Y
    global PRE_name_r_upLip_ctrl_Z

    all_rows = 1
    if all_rows > 0 and recstart == 1:

        if isInitialized:
            # name_Nose_ctrl (dataarray[0] ~ dataarray[2])
            pm.move((float(dataarray[2]) - float(PRE_name_Nose_ctrl_X)) * strengthX,
                    (float(dataarray[1]) - float(PRE_name_Nose_ctrl_Y)) * strengthY,
                    (-float(dataarray[0]) - float(PRE_name_Nose_ctrl_Z)) * strengthZ,
                    'name_Nose_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_Nose_ctrl_Z = -float(dataarray[0])
            PRE_name_Nose_ctrl_Y = dataarray[1]
            PRE_name_Nose_ctrl_X = dataarray[2]

            # name_downLip_ctrl (dataarray[3] ~ dataarray[5])
            # name_down_teeth_ctrl (dataarray[3] ~ dataarray[5])
            # name_jaw_ctrl (dataarray[3] ~ dataarray[5])

            pm.move(float((float(dataarray[5]) - float(PRE_name_downLip_ctrl_X)) * strengthX) * (-1),
                    float((float(dataarray[4]) - float(PRE_name_downLip_ctrl_Y)) * strengthY) * (-1),       # 이거 -1 왜 곱했었지?
                    float((-float(dataarray[3]) - float(PRE_name_downLip_ctrl_Z)) * strengthZ),        # 여기 -1을 곱해야 할까?
                    'name_downLip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            # 치아는 입술보단 덜 움직이므로 x 좌표 움직임에 0.8 곱함.
            pm.move(float((float(dataarray[5]) - float(PRE_name_downLip_ctrl_X)) * strengthX) * (-0.5),
                    float((float(dataarray[4]) - float(PRE_name_downLip_ctrl_Y)) * strengthY) * (0.8),      # downLip은 -1 곱하는데 downTeeth는 안곱해도 되나?
                    # 입술이 앞뒤로 움직인다고 이빨도 앞뒤로 움직이는건 아니므로 이빨은 z 움직임 무시.
                    'name_down_teeth_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            # 턱도 아랫입술과 같이 움직임
            pm.move(float((float(dataarray[5]) - float(PRE_name_downLip_ctrl_X)) * strengthX) * (-0.5),
                    float((float(dataarray[4]) - float(PRE_name_downLip_ctrl_Y)) * strengthY) * (0.8),
                    # 입술이 앞뒤로 움직인다고 턱이 앞뒤로 움직이는건 아니므로 턱은 z 움직임 무시.
                    'name_jaw_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_downLip_ctrl_Z = -float(dataarray[3])
            PRE_name_downLip_ctrl_Y = dataarray[4]
            PRE_name_downLip_ctrl_X = dataarray[5]

            ############################################################################################
            # name_down_teeth_ctrl (dataarray[3] ~ dataarray[5])

            # global PRE_name_downLip_ctrl_X
            # global PRE_name_downLip_ctrl_Y
            # global PRE_name_downLip_ctrl_Z

            # 치아는 입술보단 덜 움직이므로 x 좌표 움직임에 0.7 곱함.
            # pm.move(float((float(dataarray[5]) - float(PRE_name_downLip_ctrl_X)) * strengthX) * (-0.7), float((float(dataarray[4]) - float(PRE_name_downLip_ctrl_Y)) * strengthY) * (-1), 'name_down_teeth_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

            # PRE_name_downLip_ctrl_Z = -float(dataarray[3])
            # PRE_name_downLip_ctrl_Y = dataarray[4]
            # PRE_name_downLip_ctrl_X = dataarray[5]
            ############################################################################################

            # name_l_downLip_ctrl (dataarray[6] ~ dataarray[8])

            pm.move(float((float(dataarray[8]) - float(PRE_name_l_downLip_ctrl_X)) * strengthX) * (-1),
                    float((float(dataarray[7]) - float(PRE_name_l_downLip_ctrl_Y)) * strengthY) * (-1),
                    (-float(dataarray[6]) - float(PRE_name_l_downLip_ctrl_Z)) * strengthZ,      # 마찬가지로 -1 안움직여도 될 지..
                    'name_l_downLip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_l_downLip_ctrl_Z = -float(dataarray[6])
            PRE_name_l_downLip_ctrl_Y = dataarray[7]
            PRE_name_l_downLip_ctrl_X = dataarray[8]

            # name_r_downLip_ctrl (dataarray[9] ~ dataarray[11])

            pm.move((float(dataarray[11]) - float(PRE_name_r_downLip_ctrl_X)) * strengthX,
                    float((float(dataarray[10]) - float(PRE_name_r_downLip_ctrl_Y)) * strengthY) * (-1),
                    (-float(dataarray[9]) - float(PRE_name_r_downLip_ctrl_Z)) * strengthZ,      # 마찬가지로 -1 안움직여도 될 지..
                    'name_r_downLip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_r_downLip_ctrl_Z = -float(dataarray[9])
            PRE_name_r_downLip_ctrl_Y = dataarray[10]
            PRE_name_r_downLip_ctrl_X = dataarray[11]

            # name_l_up_cheek_ctrl (dataarray[12] ~ dataarray[14])

            pm.move((float(dataarray[14]) - float(PRE_name_l_up_cheek_ctrl_X)) * strengthX,
                    (float(dataarray[13]) - float(PRE_name_l_up_cheek_ctrl_Y)) * strengthY,
                    (-float(dataarray[12]) - float(PRE_name_l_up_cheek_ctrl_Z)) * strengthZ,
                    'name_l_up_cheek_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_l_up_cheek_ctrl_Z = -float(dataarray[12])
            PRE_name_l_up_cheek_ctrl_Y = dataarray[13]
            PRE_name_l_up_cheek_ctrl_X = dataarray[14]

            # name_l_cheek_ctrl (dataarray[15] ~ dataarray[17])

            pm.move((float(dataarray[17]) - float(PRE_name_l_cheek_ctrl_X)) * strengthX,
                    (float(dataarray[16]) - float(PRE_name_l_cheek_ctrl_Y)) * strengthY,
                    (-float(dataarray[15]) - float(PRE_name_l_cheek_ctrl_Z)) * strengthZ,
                    'name_l_cheek_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_l_cheek_ctrl_Z = -float(dataarray[15])
            PRE_name_l_cheek_ctrl_Y = dataarray[16]
            PRE_name_l_cheek_ctrl_X = dataarray[17]

            # name_l_Nose_ctrl (dataarray[18] ~ dataarray[20])

            pm.move((float(dataarray[20]) - float(PRE_name_l_Nose_ctrl_X)) * strengthX,
                    (float(dataarray[19]) - float(PRE_name_l_Nose_ctrl_Y)) * strengthY,
                    (-float(dataarray[18]) - float(PRE_name_l_Nose_ctrl_Z)) * strengthZ, 'name_l_Nose_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_l_Nose_ctrl_Z = -float(dataarray[18])
            PRE_name_l_Nose_ctrl_Y = dataarray[19]
            PRE_name_l_Nose_ctrl_X = dataarray[20]

            # name_l_Lip_ctrl (dataarray[21] ~ dataarray[23])

            pm.move(float((float(dataarray[23]) - float(PRE_name_l_Lip_ctrl_X)) * strengthX) * (-1),    # 여긴 왜 -1 곱했더라..?
                    (float(dataarray[22]) - float(PRE_name_l_Lip_ctrl_Y)) * strengthY,
                    (-float(dataarray[21]) - float(PRE_name_l_Lip_ctrl_Z)) * strengthZ,
                    'name_l_Lip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_l_Lip_ctrl_Z = -float(dataarray[21])
            PRE_name_l_Lip_ctrl_Y = dataarray[22]
            PRE_name_l_Lip_ctrl_X = dataarray[23]

            # name_l_jaw_cheek_ctrl (dataarray[24] ~ dataarray[26])

            pm.move((float(dataarray[26]) - float(PRE_name_l_jaw_cheek_ctrl_X)) * strengthX,
                    (float(dataarray[25]) - float(PRE_name_l_jaw_cheek_ctrl_Y)) * strengthY,
                    (-float(dataarray[24]) - float(PRE_name_l_jaw_cheek_ctrl_Z)) * strengthZ,
                    'name_l_jaw_cheek_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_l_jaw_cheek_ctrl_Z = -float(dataarray[24])
            PRE_name_l_jaw_cheek_ctrl_Y = dataarray[25]
            PRE_name_l_jaw_cheek_ctrl_X = dataarray[26]

            # name_l_nose_cheek_ctrl (dataarray[27] ~ dataarray[29])

            pm.move((float(dataarray[29]) - float(PRE_name_l_nose_cheek_ctrl_X)) * strengthX,
                    (float(dataarray[28]) - float(PRE_name_l_nose_cheek_ctrl_Y)) * strengthY,
                    (-float(dataarray[27]) - float(PRE_name_l_nose_cheek_ctrl_Z)) * strengthZ,
                    'name_l_nose_cheek_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_l_nose_cheek_ctrl_Z = -float(dataarray[27])
            PRE_name_l_nose_cheek_ctrl_Y = dataarray[28]
            PRE_name_l_nose_cheek_ctrl_X = dataarray[29]

            # name_r_up_cheek_ctrl (dataarray[30] ~ dataarray[32])

            pm.move((float(dataarray[32]) - float(PRE_name_r_up_cheek_ctrl_X)) * strengthX,
                    (float(dataarray[31]) - float(PRE_name_r_up_cheek_ctrl_Y)) * strengthY,
                    (-float(dataarray[30]) - float(PRE_name_r_up_cheek_ctrl_Z)) * strengthZ,
                    'name_r_up_cheek_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_r_up_cheek_ctrl_Z = -float(dataarray[30])
            PRE_name_r_up_cheek_ctrl_Y = dataarray[31]
            PRE_name_r_up_cheek_ctrl_X = dataarray[32]

            # name_r_cheek_ctrl (dataarray[33] ~ dataarray[35])

            pm.move((float(dataarray[35]) - float(PRE_name_r_cheek_ctrl_X)) * strengthX,
                    (float(dataarray[34]) - float(PRE_name_r_cheek_ctrl_Y)) * strengthY,
                    (-float(dataarray[33]) - float(PRE_name_r_cheek_ctrl_Z)) * strengthZ,
                    'name_r_cheek_ctrl',relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_r_cheek_ctrl_Z = -float(dataarray[33])
            PRE_name_r_cheek_ctrl_Y = dataarray[34]
            PRE_name_r_cheek_ctrl_X = dataarray[35]

            # name_r_Nose_ctrl (dataarray[36] ~ dataarray[38])

            pm.move((float(dataarray[38]) - float(PRE_name_r_Nose_ctrl_X)) * strengthX,
                    (float(dataarray[37]) - float(PRE_name_r_Nose_ctrl_Y)) * strengthY,
                    (-float(dataarray[36]) - float(PRE_name_r_Nose_ctrl_Z)) * strengthZ,
                    'name_r_Nose_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_r_Nose_ctrl_Z = -float(dataarray[36])
            PRE_name_r_Nose_ctrl_Y = dataarray[37]
            PRE_name_r_Nose_ctrl_X = dataarray[38]

            # name_r_Lip_ctrl (dataarray[39] ~ dataarray[41])

            pm.move((float(dataarray[41]) - float(PRE_name_r_Lip_ctrl_X)) * strengthX,
                    (float(dataarray[40]) - float(PRE_name_r_Lip_ctrl_Y)) * strengthY,
                    (-float(dataarray[39]) - float(PRE_name_r_Lip_ctrl_Z)) * strengthZ,
                    'name_r_Lip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_r_Lip_ctrl_Z = -float(dataarray[39])
            PRE_name_r_Lip_ctrl_Y = dataarray[40]
            PRE_name_r_Lip_ctrl_X = dataarray[41]

            # name_r_jaw_cheek_ctrl (dataarray[42] ~ dataarray[44])

            pm.move((float(dataarray[44]) - float(PRE_name_r_jaw_cheek_ctrl_X)) * strengthX,
                    (float(dataarray[43]) - float(PRE_name_r_jaw_cheek_ctrl_Y)) * strengthY,
                    (-float(dataarray[42]) - float(PRE_name_r_jaw_cheek_ctrl_Z)) * strengthZ,
                    'name_r_jaw_cheek_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_r_jaw_cheek_ctrl_Z = -float(dataarray[42])
            PRE_name_r_jaw_cheek_ctrl_Y = dataarray[43]
            PRE_name_r_jaw_cheek_ctrl_X = dataarray[44]

            # name_r_nose_cheek_ctrl (dataarray[45] ~ dataarray[47])

            pm.move((float(dataarray[47]) - float(PRE_name_r_nose_cheek_ctrl_X)) * strengthX,
                    (float(dataarray[46]) - float(PRE_name_r_nose_cheek_ctrl_Y)) * strengthY,
                    (-float(dataarray[45]) - float(PRE_name_r_nose_cheek_ctrl_Z)) * strengthZ,
                    'name_r_nose_cheek_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_r_nose_cheek_ctrl_Z = -float(dataarray[45])
            PRE_name_r_nose_cheek_ctrl_Y = dataarray[46]
            PRE_name_r_nose_cheek_ctrl_X = dataarray[47]

            # name_l_down_eye_border_ctrl (dataarray[48] ~ dataarray[50])

            pm.move((float(dataarray[50]) - float(PRE_name_l_down_eye_border_ctrl_X)) * strengthX,
                    (float(dataarray[49]) - float(PRE_name_l_down_eye_border_ctrl_Y)) * strengthY,
                    (-float(dataarray[48]) - float(PRE_name_l_down_eye_border_ctrl_Z)) * strengthZ,
                    'name_l_down_eye_border_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_l_down_eye_border_ctrl_Z = -float(dataarray[48])
            PRE_name_l_down_eye_border_ctrl_Y = dataarray[49]
            PRE_name_l_down_eye_border_ctrl_X = dataarray[50]

            # name_r_down_eye_border_ctrl (dataarray[51] ~ dataarray[53])

            pm.move((float(dataarray[53]) - float(PRE_name_r_down_eye_border_ctrl_X)) * strengthX,
                    (float(dataarray[52]) - float(PRE_name_r_down_eye_border_ctrl_Y)) * strengthY,
                    (-float(dataarray[51]) - float(PRE_name_r_down_eye_border_ctrl_Z)) * strengthZ,
                    'name_r_down_eye_border_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_r_down_eye_border_ctrl_Z = -float(dataarray[51])
            PRE_name_r_down_eye_border_ctrl_Y = dataarray[52]
            PRE_name_r_down_eye_border_ctrl_X = dataarray[53]

            # name_upLip_ctrl (dataarray[54] ~ dataarray[56])
            # name_up_teeth_ctrl (dataarray[54] ~ dataarray[56])

            pm.move(float((float(dataarray[56]) - float(PRE_name_upLip_ctrl_X)) * strengthX) * (-1),        # 왜 X만 -1을 곱했더라...?
                    (float(dataarray[55]) - float(PRE_name_upLip_ctrl_Y)) * strengthY,
                    (-float(dataarray[54]) - float(PRE_name_upLip_ctrl_Z)) * strengthZ,
                    'name_upLip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            # 이빨은 입술보단 좌우로 덜 움직이니 x좌표 움직임에 0.5곱함.
            pm.move(float((float(dataarray[56]) - float(PRE_name_upLip_ctrl_X)) * strengthX) * (-0.5),
                    (float(dataarray[55]) - float(PRE_name_upLip_ctrl_Y)) * strengthY,
                    # 입술이 앞뒤로 움직인다고 이빨도 앞뒤로 움직이는건 아니므로 z움직임 무시.
                    'name_up_teeth_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_upLip_ctrl_Z = -float(dataarray[54])
            PRE_name_upLip_ctrl_Y = dataarray[55]
            PRE_name_upLip_ctrl_X = dataarray[56]

            ############################################################################################
            # name_up_teeth_ctrl (dataarray[54] ~ dataarray[56])

            # global PRE_name_upLip_ctrl_X
            # global PRE_name_upLip_ctrl_Y
            # global PRE_name_upLip_ctrl_Z

            # 이빨은 입술보단 좌우로 덜 움직이니 x좌표 움직임에 0.5곱함.
            # pm.move(float((float(dataarray[56]) - float(PRE_name_upLip_ctrl_X)) * strengthX) * (-0.5), (float(dataarray[55]) - float(PRE_name_upLip_ctrl_Y)) * strengthY, 'name_up_teeth_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

            # PRE_name_upLip_ctrl_Z = -float(dataarray[54])
            # PRE_name_upLip_ctrl_Y = dataarray[55]
            # PRE_name_upLip_ctrl_X = dataarray[56]
            ############################################################################################

            # name_l_upLip_ctrl (dataarray[57] ~ dataarray[59])

            pm.move(float((float(dataarray[59]) - float(PRE_name_l_upLip_ctrl_X)) * strengthX) * (-1),
                    (float(dataarray[58]) - float(PRE_name_l_upLip_ctrl_Y)) * strengthY,
                    (-float(dataarray[57]) - float(PRE_name_l_upLip_ctrl_Z)) * strengthZ,
                    'name_l_upLip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_l_upLip_ctrl_Z = -float(dataarray[57])
            PRE_name_l_upLip_ctrl_Y = dataarray[58]
            PRE_name_l_upLip_ctrl_X = dataarray[59]

            # name_r_upLip_ctrl (dataarray[60] ~ dataarray[62])

            pm.move((float(dataarray[62]) - float(PRE_name_r_upLip_ctrl_X)) * strengthX,
                    (float(dataarray[61]) - float(PRE_name_r_upLip_ctrl_Y)) * strengthY,
                    (-float(dataarray[60]) - float(PRE_name_r_upLip_ctrl_Z)) * strengthZ,
                    'name_r_upLip_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True)

            PRE_name_r_upLip_ctrl_Z = -float(dataarray[60])
            PRE_name_r_upLip_ctrl_Y = dataarray[61]
            PRE_name_r_upLip_ctrl_X = dataarray[62]
        else:
            # name_Nose_ctrl (dataarray[0] ~ dataarray[2])

            PRE_name_Nose_ctrl_Z = -float(dataarray[0])
            PRE_name_Nose_ctrl_Y = dataarray[1]
            PRE_name_Nose_ctrl_X = dataarray[2]

            PRE_name_downLip_ctrl_Z = -float(dataarray[3])
            PRE_name_downLip_ctrl_Y = dataarray[4]
            PRE_name_downLip_ctrl_X = dataarray[5]

            PRE_name_l_downLip_ctrl_Z = -float(dataarray[6])
            PRE_name_l_downLip_ctrl_Y = dataarray[7]
            PRE_name_l_downLip_ctrl_X = dataarray[8]

            # name_r_downLip_ctrl (dataarray[9] ~ dataarray[11])

            PRE_name_r_downLip_ctrl_Z = -float(dataarray[9])
            PRE_name_r_downLip_ctrl_Y = dataarray[10]
            PRE_name_r_downLip_ctrl_X = dataarray[11]

            # name_l_up_cheek_ctrl (dataarray[12] ~ dataarray[14])

            PRE_name_l_up_cheek_ctrl_Z = -float(dataarray[12])
            PRE_name_l_up_cheek_ctrl_Y = dataarray[13]
            PRE_name_l_up_cheek_ctrl_X = dataarray[14]

            # name_l_cheek_ctrl (dataarray[15] ~ dataarray[17])

            PRE_name_l_cheek_ctrl_Z = -float(dataarray[15])
            PRE_name_l_cheek_ctrl_Y = dataarray[16]
            PRE_name_l_cheek_ctrl_X = dataarray[17]

            # name_l_Nose_ctrl (dataarray[18] ~ dataarray[20])

            PRE_name_l_Nose_ctrl_Z = -float(dataarray[18])
            PRE_name_l_Nose_ctrl_Y = dataarray[19]
            PRE_name_l_Nose_ctrl_X = dataarray[20]

            # name_l_Lip_ctrl (dataarray[21] ~ dataarray[23])

            PRE_name_l_Lip_ctrl_Z = -float(dataarray[21])
            PRE_name_l_Lip_ctrl_Y = dataarray[22]
            PRE_name_l_Lip_ctrl_X = dataarray[23]

            # name_l_jaw_cheek_ctrl (dataarray[24] ~ dataarray[26])

            PRE_name_l_jaw_cheek_ctrl_Z = -float(dataarray[24])
            PRE_name_l_jaw_cheek_ctrl_Y = dataarray[25]
            PRE_name_l_jaw_cheek_ctrl_X = dataarray[26]

            # name_l_nose_cheek_ctrl (dataarray[27] ~ dataarray[29])

            PRE_name_l_nose_cheek_ctrl_Z = -float(dataarray[27])
            PRE_name_l_nose_cheek_ctrl_Y = dataarray[28]
            PRE_name_l_nose_cheek_ctrl_X = dataarray[29]

            # name_r_up_cheek_ctrl (dataarray[30] ~ dataarray[32])

            PRE_name_r_up_cheek_ctrl_Z = -float(dataarray[30])
            PRE_name_r_up_cheek_ctrl_Y = dataarray[31]
            PRE_name_r_up_cheek_ctrl_X = dataarray[32]

            # name_r_cheek_ctrl (dataarray[33] ~ dataarray[35])

            PRE_name_r_cheek_ctrl_Z = -float(dataarray[33])
            PRE_name_r_cheek_ctrl_Y = dataarray[34]
            PRE_name_r_cheek_ctrl_X = dataarray[35]

            # name_r_Nose_ctrl (dataarray[36] ~ dataarray[38])

            PRE_name_r_Nose_ctrl_Z = -float(dataarray[36])
            PRE_name_r_Nose_ctrl_Y = dataarray[37]
            PRE_name_r_Nose_ctrl_X = dataarray[38]

            # name_r_Lip_ctrl (dataarray[39] ~ dataarray[41])

            PRE_name_r_Lip_ctrl_Z = -float(dataarray[39])
            PRE_name_r_Lip_ctrl_Y = dataarray[40]
            PRE_name_r_Lip_ctrl_X = dataarray[41]

            # name_r_jaw_cheek_ctrl (dataarray[42] ~ dataarray[44])

            PRE_name_r_jaw_cheek_ctrl_Z = -float(dataarray[42])
            PRE_name_r_jaw_cheek_ctrl_Y = dataarray[43]
            PRE_name_r_jaw_cheek_ctrl_X = dataarray[44]

            # name_r_nose_cheek_ctrl (dataarray[45] ~ dataarray[47])

            PRE_name_r_nose_cheek_ctrl_Z = -float(dataarray[45])
            PRE_name_r_nose_cheek_ctrl_Y = dataarray[46]
            PRE_name_r_nose_cheek_ctrl_X = dataarray[47]

            # name_l_down_eye_border_ctrl (dataarray[48] ~ dataarray[50])

            PRE_name_l_down_eye_border_ctrl_Z = -float(dataarray[48])
            PRE_name_l_down_eye_border_ctrl_Y = dataarray[49]
            PRE_name_l_down_eye_border_ctrl_X = dataarray[50]

            # name_r_down_eye_border_ctrl (dataarray[51] ~ dataarray[53])

            PRE_name_r_down_eye_border_ctrl_Z = -float(dataarray[51])
            PRE_name_r_down_eye_border_ctrl_Y = dataarray[52]
            PRE_name_r_down_eye_border_ctrl_X = dataarray[53]

            # name_upLip_ctrl (dataarray[54] ~ dataarray[56])
            # name_up_teeth_ctrl (dataarray[54] ~ dataarray[56])

            PRE_name_upLip_ctrl_Z = -float(dataarray[54])
            PRE_name_upLip_ctrl_Y = dataarray[55]
            PRE_name_upLip_ctrl_X = dataarray[56]

            ############################################################################################
            # name_up_teeth_ctrl (dataarray[54] ~ dataarray[56])

            # global PRE_name_upLip_ctrl_X
            # global PRE_name_upLip_ctrl_Y
            # global PRE_name_upLip_ctrl_Z

            # 이빨은 입술보단 좌우로 덜 움직이니 x좌표 움직임에 0.5곱함.
            # pm.move(float((float(dataarray[56]) - float(PRE_name_upLip_ctrl_X)) * strengthX) * (-0.5), (float(dataarray[55]) - float(PRE_name_upLip_ctrl_Y)) * strengthY, 'name_up_teeth_ctrl', relative=True, objectSpace=True, worldSpaceDistance=True )

            # PRE_name_upLip_ctrl_Z = -float(dataarray[54])
            # PRE_name_upLip_ctrl_Y = dataarray[55]
            # PRE_name_upLip_ctrl_X = dataarray[56]
            ############################################################################################

            # name_l_upLip_ctrl (dataarray[57] ~ dataarray[59])

            PRE_name_l_upLip_ctrl_Z = -float(dataarray[57])
            PRE_name_l_upLip_ctrl_Y = dataarray[58]
            PRE_name_l_upLip_ctrl_X = dataarray[59]

            # name_r_upLip_ctrl (dataarray[60] ~ dataarray[62])

            PRE_name_r_upLip_ctrl_Z = -float(dataarray[60])
            PRE_name_r_upLip_ctrl_Y = dataarray[61]
            PRE_name_r_upLip_ctrl_X = dataarray[62]

            isInitialized = True;


def portData(arg):
    """
    Read the 'serial' data passed in from the commandPort
    """
    global dataarray

    dataarray = []
    # 이차 배열의 첫 배열 dataarray.
    recVal = str(arg)
    # 받은 arg 스트링을 recVal에 넣음.
    strArray = recVal.split(",")
    # recVal 스트링을 , 경계로 쪼갬.
    for i in range(0, 63):
        # 63가지의 데이터가 들어옴.
        dataarray.append(strArray[i])
        # print(strArray[i])
        # dataarray에 strArray[i] 첨부.
        # print(strArray[i])
    # createTimer(0.03, deformface)
    deformface()


def deactivateCommandPort(host, port):
    path = host + ":" + port
    active = cmds.commandPort(path, q=True)
    if active:
        cmds.commandPort(name=path, cl=True)
    else:
        print("%s is was not active" % path)

```
