# Facial rendering source scan: PrashanthaTP/wav2mov

Repository: https://github.com/PrashanthaTP/wav2mov

Pinned source revision: fabf89aec6c149b223a9d4d187f763363177abe1

Discovery topic: facial animation

Description: Speech to Facial Animation using GANs

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: setup.py

{
  "path": "setup.py",
  "blob_sha": "4bf5a1f649a8e069a07184e7fe856bfb1a9a4d86",
  "sha256": "8b8feed1581e8b6870cfb7d66a230839f6d214dd5a3818ac0261529fbf0b4444",
  "lines": 2,
  "bytes": 107,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/PrashanthaTP/wav2mov/blob/fabf89aec6c149b223a9d4d187f763363177abe1/setup.py",
  "function_signature_candidates": []
}

```text

from setuptools import setup,find_packages
setup(name="wav2mov",version="1.0.0",packages= find_packages())


```

## Source file: wav2mov/config.py

{
  "path": "wav2mov/config.py",
  "blob_sha": "3df44c0f4a049c8ed03aeb9cb215cda84880ef5c",
  "sha256": "d43f00e2357c6b16a1d1da7761483fd97ea63f97b95867df03307aa3fc723e70",
  "lines": 94,
  "bytes": 3306,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1
  },
  "url": "https://github.com/PrashanthaTP/wav2mov/blob/fabf89aec6c149b223a9d4d187f763363177abe1/wav2mov/config.py",
  "function_signature_candidates": [
    {
      "line": 14,
      "text": "def get_curr_run_str():"
    },
    {
      "line": 22,
      "text": "def __init__(self,v):"
    },
    {
      "line": 30,
      "text": "def _rectify_paths(self,val):"
    },
    {
      "line": 38,
      "text": "def _flatten_dict(self,d):"
    },
    {
      "line": 52,
      "text": "def _update_vals_from_dict(self,d:dict):"
    },
    {
      "line": 63,
      "text": "def from_json(cls,json_file_fullpath,v):"
    },
    {
      "line": 70,
      "text": "def update(self,key,value):"
    },
    {
      "line": 75,
      "text": "def __getitem__(self,item):"
    },
    {
      "line": 89,
      "text": "def get_config(v):"
    }
  ]
}

```text

"""Config"""
import json
import logging
import os
import re
import pytz
from datetime import datetime
from wav2mov.logger import get_module_level_logger

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
logger = get_module_level_logger(__name__)
logger.setLevel(logging.WARNING)

def get_curr_run_str():
    now = datetime.now().astimezone(pytz.timezone("Asia/Calcutta"))
    date,time = now.date(),now.time()
    day,month,year = date.day,date.month,date.year
    hour , minutes = time.hour,time.minute
    return 'Run_{}_{}_{}__{}_{}'.format(day,month,year,hour,minutes)

class Config :
    def __init__(self,v):
        self.vals = {'base_dir':BASE_DIR}
        self.version = get_curr_run_str()
        #these two are used to decide whether to create a path using os.makedirs
        self.fixed_paths = set()
        self.runtime_paths = set()
        self.v = v

    def _rectify_paths(self,val):
        return val % {'base_dir': self.vals['base_dir'],
                      'version': self.version,
                      'v':self.v,
                      'checkpoint_filename': 'checkpoint_'+self.version,
                      'log_filename': 'log_' + self.version
                      }
    
    def _flatten_dict(self,d):
        flattened= {}
        for k,v in d.items():
            if isinstance(v,dict):
                inner_d = self._flatten_dict(v)
                for k_inner in inner_d.keys():
                    if k== 'fixed': self.fixed_paths.add(k_inner)
                    else: self.runtime_paths.add(k_inner)
                    
                flattened = {**flattened,**inner_d}
            else:
                flattened[k] = v
        return flattened
                
    def _update_vals_from_dict(self,d:dict):
        d = self._flatten_dict(d)
        for key,val in d.items():
            if isinstance(val,str):
                value = self._rectify_paths(val)
                
            else:
                value = val
            self.vals[key] = value
            
    @classmethod
    def from_json(cls,json_file_fullpath,v):
        with open(json_file_fullpath,'r') as file:
            configs = json.load(file)
            obj = cls(v)
            obj._update_vals_from_dict(configs)
            return obj
        
    def update(self,key,value):
        if key in self.vals:
            logger.warning(f'Updating existing parameter {key} : changing from {self.vals[key]} with {value}')
        self.vals[key] = value 
        
    def __getitem__(self,item):
        if item not in self.vals:
            logger.error(f'{self.__class__.__name__}  object has no key called {item}')
            raise KeyError('No key called ', item)
        if item in self.fixed_paths:
            return self.vals[item]
        if os.sep != '\\':
              self.vals[item] = re.sub(r'(\\)+', os.sep, self.vals[item])
        if 'fullpath' in item or 'dir' in item:
            path = os.path.dirname(self.vals[item]) if '.' in os.path.basename(self.vals[item]) else self.vals[item]
            os.makedirs(path,exist_ok=True)
        logger.debug(f'accessing {self.vals[item]}')
        return self.vals[item]
    
def get_config(v):
    config = Config.from_json(os.path.join(BASE_DIR,'config.json'),v)
    return config

if __name__=='__main__':
    pass


```

## Source file: wav2mov/logger.py

{
  "path": "wav2mov/logger.py",
  "blob_sha": "80e802c0cd49ac0c47daa7daa0904b8c42b76957",
  "sha256": "57df9d90d6097cfa1d4d9b2174beb6455fdf1e5850ba7fcf65f2d769de465061",
  "lines": 168,
  "bytes": 5687,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/PrashanthaTP/wav2mov/blob/fabf89aec6c149b223a9d4d187f763363177abe1/wav2mov/logger.py",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "def get_module_level_logger(name):"
    },
    {
      "line": 23,
      "text": "def add_fields(self,log_record,record,message_dict):"
    },
    {
      "line": 41,
      "text": "def __init__(self,runs_dir):"
    },
    {
      "line": 44,
      "text": "def create_writer(self,name):"
    },
    {
      "line": 47,
      "text": "def add_writer(self, name, writer):"
    },
    {
      "line": 50,
      "text": "def add_writers(self,names):"
    },
    {
      "line": 54,
      "text": "def add_scalar(self,writer_name,tag,scalar,global_step):"
    },
    {
      "line": 61,
      "text": "def add_scalars(self,d,global_step):"
    },
    {
      "line": 70,
      "text": "def __init__(self, name):"
    },
    {
      "line": 79,
      "text": "def __add_handler(self, handler: logging.Handler):"
    },
    {
      "line": 84,
      "text": "def __get_formatter(cls, fmt: str):"
    },
    {
      "line": 88,
      "text": "def __get_json_formatter(cls,fmt):"
    },
    {
      "line": 91,
      "text": "def __json_log_begin(self):"
    },
    {
      "line": 98,
      "text": "def __json_log_end(self):"
    },
    {
      "line": 104,
      "text": "def add_filehandler(self, log_fullpath, fmt:str=None,in_json=False):"
    },
    {
      "line": 126,
      "text": "def add_console_handler(self, fmt: str = None):"
    },
    {
      "line": 133,
      "text": "def __add_comma(self):"
    },
    {
      "line": 141,
      "text": "def debug(self):"
    },
    {
      "line": 146,
      "text": "def info(self):"
    },
    {
      "line": 151,
      "text": "def warning(self):"
    },
    {
      "line": 156,
      "text": "def error(self):"
    },
    {
      "line": 161,
      "text": "def exception(self):"
    },
    {
      "line": 165,
      "text": "def cleanup(self):"
    }
  ]
}

```text

import logging
import os
import re
from datetime import datetime
from torch.utils.tensorboard.writer import SummaryWriter
from pythonjsonlogger import jsonlogger

logging.basicConfig(level=logging.ERROR,format="%(levelname)-5s : %(name)s : %(asctime)s | %(msg)s ")
TIME_FORMAT = "%b %d,%Y %H:%M:%S"
 
def get_module_level_logger(name):
    m_logger =  logging.getLogger(name)
    m_logger.setLevel(logging.DEBUG)
    m_logger.propagate = False
    handler = logging.StreamHandler()
    handler.setFormatter(logging.Formatter("%(levelname)-5s : %(name)s : %(asctime)s | %(msg)s ",TIME_FORMAT))
    m_logger.addHandler(handler)
    return m_logger


logger = get_module_level_logger(__name__)
class CustomJsonFormatter(jsonlogger.JsonFormatter):
    def add_fields(self,log_record,record,message_dict):
        # print(log_record,vars(record),message_dict)
        # for key,value in vars(record).items():
        #     print(f'{key} : {value}')
        super().add_fields(log_record,record,message_dict)
        # if not log_record.get('timestamp'):
            # now = datetime.utcnow().strftime(TIME_FORMAT)
            # log_record['timestamp'] = now
        # if log_record.get('level'):
        #     log_record['level'] = log_record['level'].upper()
        # else:
        #     log_record['level'] = record.levelname
        if log_record.get('asctime'):
            log_record['asctime'] = datetime.utcnow().strftime(TIME_FORMAT)
     
 

class TensorLogger:
    def __init__(self,runs_dir):
        self.writers = {}
        self.runs_dir = runs_dir
    def create_writer(self,name):
        self.writers[name] = SummaryWriter(os.path.join(self.runs_dir,name))
        
    def add_writer(self, name, writer):
        self.writers[name] = writer
        
    def add_writers(self,names):
        for name in names:
            self.create_writer(name)
            
    def add_scalar(self,writer_name,tag,scalar,global_step):
        if writer_name not in self.writers:
            logger.warning(f'No writer found named {writer_name}')
            self.create_writer(writer_name)
        self.writers[writer_name].add_scalar(tag,scalar,global_step)
    
    
    def add_scalars(self,d,global_step):
        for writer_name,(tag,scalar) in d.items():
            self.add_scalar(writer_name,tag,scalar,global_step)
            
            

    
    
class Logger:
    def __init__(self, name):
        self.log_fullpath = None
        self.name = name
        self.logger = logging.getLogger(name)
        self.logger.propagate = False
        self.logger.setLevel(logging.DEBUG)
        self.is_json = False
        self.is_first_log = True
        
    def __add_handler(self, handler: logging.Handler):
        handler.setLevel(logging.DEBUG)
        self.logger.addHandler(handler)

    @classmethod
    def __get_formatter(cls, fmt: str):
        return logging.Formatter(fmt,TIME_FORMAT)
    
    @classmethod
    def __get_json_formatter(cls,fmt):
        return CustomJsonFormatter(fmt=fmt)
    
    def __json_log_begin(self):
        # print(f'inside json begin')
        with open(self.log_fullpath,'a+') as file:
            # print(file.read())
            file.write('[\n')
            # print(file.read())
            
    def __json_log_end(self):
        print('writing "]" to json log')
        with open(self.log_fullpath,'a+') as file:
            file.write(']\n')
   
            
    def add_filehandler(self, log_fullpath, fmt:str=None,in_json=False):
        self.log_fullpath = log_fullpath
        if fmt is None:
            fmt = '%(levelname)-5s : %(filename)s :  %(asctime)s : line no: %(lineno)d : %(message)s'
        if in_json:
            self.is_json = True
            folder = os.path.dirname(self.log_fullpath)
            filename= os.path.basename(self.log_fullpath)
            filename = filename.split('.')[0] + '.json'
           
            self.log_fullpath = os.path.join(folder,filename)
            
            self.__json_log_begin()
            file_handler = logging.FileHandler(self.log_fullpath)
            file_handler.setFormatter(self.__get_json_formatter(fmt))
        else:
            file_handler = logging.FileHandler(self.log_fullpath)
            file_handler.setFormatter(self.__get_formatter(fmt))
        # self.log_fullpath = re.sub(r'(\\)',os.sep,self.log_fullpath)
        os.makedirs(os.path.dirname(self.log_fullpath), exist_ok=True)
        self.__add_handler(file_handler)

    def add_console_handler(self, fmt: str = None):
        if fmt is None:
            fmt = "%(levelname)-5s : %(filename)s :  %(asctime)s : line no: %(lineno)d : %(message)s"
        console_handler = logging.StreamHandler()
        console_handler.setFormatter(self.__get_formatter(fmt))
        self.__add_handler(console_handler)

    def __add_comma(self):
        if self.is_first_log:
            self.is_first_log = False
            return
        with open(self.log_fullpath,'a+') as file:
            file.write(',\n')
            
    @property
    def debug(self):
        if self.is_json:self.__add_comma()
        return self.logger.debug
    
    @property
    def info(self):
        if self.is_json:self.__add_comma()
        return self.logger.info
    
    @property
    def warning(self):
        if self.is_json:self.__add_comma()
        return self.logger.warning
    
    @property
    def error(self):
        if self.is_json:self.__add_comma()
        return self.logger.error
    
    @property
    def exception(self):
        if self.is_json:self.__add_comma()
        return self.logger.exception
    
    def cleanup(self):
        if self.is_json:
            self.__json_log_end()
    


```

## Source file: wav2mov/params.py

{
  "path": "wav2mov/params.py",
  "blob_sha": "e79309cacd5b8601552235d2cca1fd3109560c22",
  "sha256": "7d3eaa6893de659cd69a3e100501884921baa940bd9ca94742623c7e291f2327",
  "lines": 68,
  "bytes": 1975,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1
  },
  "url": "https://github.com/PrashanthaTP/wav2mov/blob/fabf89aec6c149b223a9d4d187f763363177abe1/wav2mov/params.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def __init__(self):"
    },
    {
      "line": 12,
      "text": "def _flatten_dict(self, d):"
    },
    {
      "line": 22,
      "text": "def update(self,key,value):"
    },
    {
      "line": 28,
      "text": "def _update_vals_from_dict(self, d: dict):"
    },
    {
      "line": 34,
      "text": "def from_json(cls, json_file_fullpath):"
    },
    {
      "line": 41,
      "text": "def __getitem__(self, item):"
    },
    {
      "line": 48,
      "text": "def __repr__(self):"
    },
    {
      "line": 54,
      "text": "def set(self,key,val):"
    },
    {
      "line": 59,
      "text": "def save(self,file_fullpath):"
    }
  ]
}

```text

import json 
import os 
from wav2mov.settings import BASE_DIR
from wav2mov.logger import get_module_level_logger
logger = get_module_level_logger(__name__)
class Params:
    def __init__(self):
        self.vals = {}
       
    

    def _flatten_dict(self, d):
        flattened = {}
        for k, v in d.items():
            if isinstance(v, dict):
                inner_d = self._flatten_dict(v)
                flattened = {**flattened, **inner_d}
            else:
                flattened[k] = v
        return flattened

    def update(self,key,value):
        if key in self.vals:
            logger.warning(f'Updating existing parameter {key} : changing from {self.vals[key]} with {value}')
        
        self.vals[key] = value
        
    def _update_vals_from_dict(self, d: dict):
        self.vals = d
        # d = self._flatten_dict(d)
        # self.vals = {**self.vals,**d}

    @classmethod
    def from_json(cls, json_file_fullpath):
        with open(json_file_fullpath, 'r') as file:
            configs = json.load(file)
            obj = cls()
            obj._update_vals_from_dict(configs)
            return obj

    def __getitem__(self, item):
        if item not in self.vals:
            logger.error(f'{self.__class__.__name__} object has no key called {item}')
            raise KeyError(f'No key called {item}')
       
        return self.vals[item]

    def __repr__(self):
        fstr = ''
        for key,val in self.vals.items():
            fstr += f"{key} : {val}\n"
        return fstr
    
    def set(self,key,val):
        if key in self.vals:
            logger.warning(f'updating existing value of {key} with value {self.vals[key] } to {val}')
        self.vals[key] = val
        
    def save(self,file_fullpath):
        with open(file_fullpath,'w') as file:
            json.dump(self.vals,file)


#Singleton Object
params = Params.from_json(os.path.join(BASE_DIR, 'params.json'))

if __name__ == '__main__':
    pass


```
